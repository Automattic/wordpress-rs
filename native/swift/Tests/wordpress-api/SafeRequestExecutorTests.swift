import Foundation
import Testing
@testable import WordPressAPI
@testable import WordPressAPIInternal

@Suite("SafeRequestExecutor")
struct SafeRequestExecutorTests {

    // Regression test for #1513: `sleep(millis:)` converted milliseconds to nanoseconds with the
    // wrong factor (`millis * 1_000` instead of `* 1_000_000`), so it slept 1000× too short and
    // `RetryAfterMiddleware` never actually honored a `Retry-After` backoff.
    @Test("sleep(millis:) waits for approximately the requested duration")
    func testSleepHonorsMillisecondDuration() async {
        let executor = WpRequestExecutor(urlSession: .shared)

        let requestedMillis: UInt64 = 200
        let clock = ContinuousClock()
        let start = clock.now
        await executor.sleep(millis: requestedMillis)
        let elapsed = start.duration(to: clock.now)

        // `Task.sleep` waits *at least* the requested duration, so bound both sides. The 150 ms
        // floor catches the old 1000×-too-short bug (~0.2 ms for a 200 ms request); the 2 s
        // ceiling catches the symmetric slip (e.g. `.seconds` in place of `.milliseconds`, ~200 s)
        // while staying well clear of scheduling jitter, which is milliseconds, not seconds.
        #expect(elapsed >= .milliseconds(150))
        #expect(elapsed < .seconds(2))
    }

    // Regression tests for #1756. `URLSession` answers a `data:`, `about:` or `file:` address with a
    // plain `URLResponse`, not an `HTTPURLResponse`, and `WpNetworkResponse.init` trapped on it. A site
    // that advertised such an address as its API root crashed the app.
    @Test(
        "A non-web address fails the request instead of being sent",
        arguments: ["data:application/json,{}", "about:blank", "file:///etc/hosts", "ftp://example.com/wp-json/"]
    )
    func testNonWebAddressIsRejected(address: String) async {
        let session = URLSession(configuration: .ephemeral)
        defer { session.finishTasksAndInvalidate() }
        let executor = WpRequestExecutor(urlSession: session)

        let result = await executor.perform(BuiltRequest(address: address))

        guard case .failure(.RequestExecutionFailed(_, _, let reason, _, _)) = result else {
            Issue.record("Expected a RequestExecutionFailed failure for \(address), got \(result)")
            return
        }

        guard case .genericError = reason else {
            Issue.record("Expected .genericError for \(address), got \(reason)")
            return
        }
    }

    // A `URLProtocol` an app registers can still answer with a plain `URLResponse`. That is a failed
    // request, not a reason to crash.
    @Test("A non-HTTP response fails the request instead of crashing")
    func testNonHttpResponseIsAFailure() async {
        let executor = WpRequestExecutor(urlSession: .shared)

        let result = await executor.perform(NonHttpResponseRequest())

        guard case .failure(.RequestExecutionFailed(_, _, let reason, _, _)) = result else {
            Issue.record("Expected a RequestExecutionFailed failure, got \(result)")
            return
        }

        guard case .genericError = reason else {
            Issue.record("Expected .genericError, got \(reason)")
            return
        }
    }

    // End-to-end companion: an app-supplied `file:` API root, sent through the production
    // `WpNetworkRequest.perform` and back through the Rust client. The file exists, so without the
    // check `URLSession` reads it and answers with a plain `URLResponse`.
    @Test("A non-web API root surfaces end-to-end as an error instead of crashing")
    func testNonWebApiRootSurfacesEndToEnd() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let apiRoot = directory.appendingPathComponent("wp-json")
        try Data("{}".utf8).write(to: apiRoot)

        let session = URLSession(configuration: .ephemeral)
        defer { session.finishTasksAndInvalidate() }

        let api = try WordPressAPI(
            siteInfo: .selfHosted(
                siteUrl: ParsedUrl.parse(input: directory.absoluteString),
                apiRoot: ParsedUrl.parse(input: apiRoot.absoluteString)
            ),
            authenticationProvider: .none(),
            executor: WpRequestExecutor(urlSession: session),
            middlewarePipeline: .default,
            appNotifier: nil
        )

        do {
            _ = try await api.apiRoot.get()
            Issue.record("Expected the request to fail, but it succeeded")
        } catch {
            let reason = (error as? CarriesRequestExecutionErrorReason)?.executionErrorReason
            guard case .some(.genericError) = reason else {
                Issue.record("Expected .genericError, got \(String(describing: reason)) (error: \(error))")
                return
            }
        }
    }

    // Regression test for #1491: a URLSession timeout (`URLError.timedOut`) had no branch in
    // `perform(_:)`, so it fell through to `.genericError` and `HttpTimeoutError` was unreachable on
    // Apple platforms — even though reqwest (`is_timeout()`) and Kotlin (`SocketTimeoutException`)
    // both classify their equivalent. Drive a real request through a `URLProtocol` that never
    // responds so URLSession's own timeout fires, and assert the reason is `.httpTimeoutError`.
    //
    // Compiled out on Linux: the stub leans on `URLProtocol`/`URLSession` timeout machinery that
    // swift-corelibs-foundation handles differently, and the classification bug is Apple-only.
    // `.timeLimit` bounds a hang if a future toolchain ever stops enforcing the request timeout.
    //
    // Excluded on watchOS (this test and its end-to-end companion below): on the watchOS 27.2
    // simulator the session's `protocolClasses` stub isn't dependably consulted when the full suite
    // runs, so the request reaches the real `example.com` and gets a 404 instead of timing out. The
    // classification is platform-agnostic and stays covered on macOS, iOS, and tvOS. The watchOS
    // simulator leg of build #6883 caught this.
    #if !os(Linux)
    #if !os(watchOS)
    @Test("A URLSession timeout is classified as .httpTimeoutError", .timeLimit(.minutes(1)))
    func testTimeoutIsClassifiedAsHttpTimeoutError() async throws {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [NeverRespondingURLProtocol.self]
        // Fail fast instead of waiting URLSession's 60 s default request timeout.
        configuration.timeoutIntervalForRequest = 0.3
        let session = URLSession(configuration: configuration)
        defer { session.finishTasksAndInvalidate() }
        let executor = WpRequestExecutor(urlSession: session)

        let result = await executor.perform(TimingOutRequest())

        guard case .failure(let error) = result,
            case .RequestExecutionFailed(_, _, let reason, _, _) = error
        else {
            Issue.record("Expected a RequestExecutionFailed failure, got \(result)")
            return
        }

        guard case .httpTimeoutError = reason else {
            Issue.record("Expected .httpTimeoutError, got \(reason)")
            return
        }
    }

    // End-to-end companion to the test above. The test above drives the classification switch in
    // isolation; this one drives a real `WordPressAPI` request through the production
    // `WpNetworkRequest.perform` machinery (the `withCheckedContinuation` +
    // `dataTask(completionHandler:)` + `withTaskCancellationHandler` path) and back through the
    // Rust client, asserting the timeout surfaces to the caller as a `WpApiError` carrying
    // `.httpTimeoutError`. It guards the whole chain the stub cannot reach — so a future refactor of
    // the continuation/completion path that re-drops a timeout to `.genericError` is caught here.
    // Mirrors Kotlin's `MockWebServer` + `SocketPolicy.NO_RESPONSE` end-to-end test.
    @Test("A URLSession timeout surfaces end-to-end as .httpTimeoutError", .timeLimit(.minutes(1)))
    func testTimeoutSurfacesEndToEndAsHttpTimeoutError() async throws {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [NeverRespondingURLProtocol.self]
        configuration.timeoutIntervalForRequest = 0.3
        let session = URLSession(configuration: configuration)
        defer { session.finishTasksAndInvalidate() }

        let api = try WordPressAPI(
            siteInfo: .selfHosted(
                siteUrl: ParsedUrl.parse(input: "https://example.com"),
                apiRoot: ParsedUrl.parse(input: "https://example.com/wp-json")
            ),
            authenticationProvider: .none(),
            executor: WpRequestExecutor(urlSession: session),
            middlewarePipeline: .default,
            appNotifier: nil
        )

        do {
            _ = try await api.apiRoot.get()
            Issue.record("Expected the request to time out, but it succeeded")
        } catch {
            let reason = (error as? CarriesRequestExecutionErrorReason)?.executionErrorReason
            guard case .some(.httpTimeoutError) = reason else {
                Issue.record("Expected .httpTimeoutError, got \(String(describing: reason)) (error: \(error))")
                return
            }
        }
    }
    #endif // !os(watchOS)

    // End-to-end companion for `MediaFileUnreadable` (#1546), mirroring the timeout test above and
    // Kotlin's `MockWebServer` executor test. A directory at the upload path passes field
    // construction (`attributesOfItem` is a `stat`, needing no read permission) but fails the stream
    // read (EISDIR) — the deterministic, uid-independent sibling of a genuine mid-read. Serialization
    // happens before any network I/O, so no `URLProtocol` is needed. This guards the chain the
    // isolated `MultipartFormTests` can't reach — `WpMultipartFormRequest.perform`'s do/catch, the
    // `asRequestExecutionError` mapping, and the Rust round-trip — so a future refactor that re-drops
    // the failure to `.genericError` is caught here.
    @Test("A mid-read serialization failure surfaces end-to-end as .MediaFileUnreadable", .timeLimit(.minutes(1)))
    func testMediaFileUnreadableSurfacesEndToEnd() async throws {
        // A directory opens (`stat` succeeds) yet can't be read as a file. `chmod 000` would be
        // bypassed when tests run as root (common in CI); a directory's EISDIR is enforced for every uid.
        let directoryPath = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true).path
        try FileManager.default.createDirectory(atPath: directoryPath, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(atPath: directoryPath) }

        // Serialization fails before the upload, so the session is never used for I/O; the short
        // request timeout only bounds a hang if a regression ever lets the request reach the network.
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = 0.3
        let session = URLSession(configuration: configuration)
        defer { session.finishTasksAndInvalidate() }

        let api = try WordPressAPI(
            siteInfo: .selfHosted(
                siteUrl: ParsedUrl.parse(input: "https://example.com"),
                apiRoot: ParsedUrl.parse(input: "https://example.com/wp-json")
            ),
            authenticationProvider: .none(),
            executor: WpRequestExecutor(urlSession: session),
            middlewarePipeline: .default,
            appNotifier: nil
        )

        await #expect(
            throws: WpApiError.MediaFileUnreadable(filePath: directoryPath),
            performing: {
                _ = try await api.media.create(params: .init(filePath: directoryPath))
            }
        )
    }

    // Regression test for #1501: three `URLError` codes that mean the device can't use the network
    // right now — cellular data disallowed (`.dataNotAllowed`), international roaming off
    // (`.internationalRoamingOff`), and an active call holding a single-radio device
    // (`.callIsActive`) — had no branch in `errorIsDeviceIsOffline`, so they fell through to the
    // catch-all `.genericError` instead of `.deviceIsOfflineError`. Drive each code through the real
    // URLSession completion path via a `URLProtocol` that fails the request with it, and assert the
    // reason is `.deviceIsOfflineError`. These codes are produced by device state a test can't set
    // (cellular policy, roaming, an in-progress call), so injecting the `URLError` is the only way to
    // exercise the branch.
    //
    // Excluded on watchOS: watchOS's URL loading system does not faithfully deliver these
    // cellular-radio codes when they are *injected* via `URLProtocolClient`, so the stub can't drive
    // the branch on that platform. The classifier
    // itself is platform-agnostic (`errorIsDeviceIsOffline` has no `#if`), and is exercised on macOS,
    // iOS, and tvOS. The watchOS simulator leg of build #6181 caught this.
    #if !os(watchOS)
    @Test(
        "OS 'can't use the network right now' codes are classified as .deviceIsOfflineError",
        arguments: [URLError.Code.dataNotAllowed, .internationalRoamingOff, .callIsActive]
    )
    func testDeviceCannotUseNetworkCodesAreClassifiedAsDeviceIsOffline(code: URLError.Code) async throws {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [FailingURLProtocol.self]
        let session = URLSession(configuration: configuration)
        defer { session.finishTasksAndInvalidate() }
        let executor = WpRequestExecutor(urlSession: session)

        let result = await executor.perform(FailingRequest(code: code))

        guard case .failure(let error) = result,
            case .RequestExecutionFailed(_, _, let reason, _, _) = error
        else {
            Issue.record("Expected a RequestExecutionFailed failure for \(code), got \(result)")
            return
        }

        guard case .deviceIsOfflineError = reason else {
            Issue.record("Expected .deviceIsOfflineError for \(code), got \(reason)")
            return
        }
    }
    #endif // !os(watchOS)
    #endif
}

/// A `NetworkRequestContent` that builds its request with the production `buildURLRequest`, then sends
/// it through the executor's session.
private struct BuiltRequest: NetworkRequestContent {
    let address: String

    func requestId() -> String { "1756-non-web-address" }
    func method() -> RequestMethod { .get }
    func url() -> WpEndpointUrl { address }
    func headerMap() -> WpNetworkHeaderMap { .empty }
    func encodeBody(into _: inout URLRequest) throws {}

    func perform(
        in session: URLSession,
        withAdditionalHeaders headers: [String: String],
        delegate _: URLSessionTaskDelegate?
    ) async throws -> (Data, URLResponse) {
        try await session.data(for: buildURLRequest(additionalHeaders: headers))
    }
}

/// A `NetworkRequestContent` answered with a plain `URLResponse`, as a `URLProtocol` an app registers
/// may answer.
private struct NonHttpResponseRequest: NetworkRequestContent {
    func requestId() -> String { "1756-non-http-response" }
    func method() -> RequestMethod { .get }
    func url() -> WpEndpointUrl { "https://example.com/wp-json/" }
    func headerMap() -> WpNetworkHeaderMap { .empty }
    func encodeBody(into _: inout URLRequest) throws {}

    func perform(
        in _: URLSession,
        withAdditionalHeaders _: [String: String],
        delegate _: URLSessionTaskDelegate?
    ) async throws -> (Data, URLResponse) {
        let body = Data("{}".utf8)
        let response = URLResponse(
            url: URL(string: url())!,
            mimeType: "application/json",
            expectedContentLength: body.count,
            textEncodingName: nil
        )
        return (body, response)
    }
}

#if !os(Linux)
// Only used by the watchOS-excluded tests above, so gated identically to avoid unused-type warnings.
#if !os(watchOS)
/// A `NetworkRequestContent` that issues its request through the executor's session, so the
/// session's own timeout produces the `URLError.timedOut` under test.
private struct TimingOutRequest: NetworkRequestContent {
    func requestId() -> String { "1491-timeout-regression" }
    func method() -> RequestMethod { .get }
    func url() -> WpEndpointUrl { "https://example.com/wp-json/" }
    func headerMap() -> WpNetworkHeaderMap { .empty }
    func encodeBody(into _: inout URLRequest) throws {}

    func perform(
        in session: URLSession,
        withAdditionalHeaders _: [String: String],
        delegate _: URLSessionTaskDelegate?
    ) async throws -> (Data, URLResponse) {
        var request = URLRequest(url: URL(string: url())!)
        // Belt-and-suspenders with `timeoutIntervalForRequest`: force a short per-request timeout.
        request.timeoutInterval = 0.3
        return try await session.data(for: request)
    }
}

/// A `URLProtocol` that accepts every request and then never responds, so the only way a task can
/// finish is by hitting its timeout — producing `URLError.timedOut` without touching the network.
private final class NeverRespondingURLProtocol: URLProtocol {
    override static func canInit(with request: URLRequest) -> Bool { true }
    override static func canonicalRequest(for request: URLRequest) -> URLRequest { request }
    override func startLoading() {
        // Intentionally never call the client.
    }
    override func stopLoading() {}
}

/// A `URLProtocol` that fails every request with the `URLError.Code` carried in a request header,
/// letting a test drive a specific OS error code through the real URLSession completion path
/// without depending on device state (cellular policy, roaming, an in-progress call) it can't set.
private final class FailingURLProtocol: URLProtocol {
    static let codeHeader = "X-Test-URLError-Code"

    override static func canInit(with request: URLRequest) -> Bool { true }
    override static func canonicalRequest(for request: URLRequest) -> URLRequest { request }
    override func startLoading() {
        let rawCode = request.value(forHTTPHeaderField: Self.codeHeader).flatMap { Int($0) }
        let code = rawCode.map { URLError.Code(rawValue: $0) } ?? .unknown
        client?.urlProtocol(self, didFailWithError: URLError(code))
    }
    override func stopLoading() {}
}

/// A `NetworkRequestContent` that issues its request through the executor's session, tagging it with
/// the `URLError.Code` for `FailingURLProtocol` to fail with.
private struct FailingRequest: NetworkRequestContent {
    let code: URLError.Code

    func requestId() -> String { "1501-device-offline-regression" }
    func method() -> RequestMethod { .get }
    func url() -> WpEndpointUrl { "https://example.com/wp-json/" }
    func headerMap() -> WpNetworkHeaderMap { .empty }
    func encodeBody(into _: inout URLRequest) throws {}

    func perform(
        in session: URLSession,
        withAdditionalHeaders _: [String: String],
        delegate _: URLSessionTaskDelegate?
    ) async throws -> (Data, URLResponse) {
        var request = URLRequest(url: URL(string: url())!)
        request.setValue(String(code.rawValue), forHTTPHeaderField: FailingURLProtocol.codeHeader)
        return try await session.data(for: request)
    }
}
#endif // !os(watchOS)
#endif
