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

    // Regression test for #1511: `buildURLRequest` force-unwrapped `URL(string:)`, so a URL
    // Foundation couldn't parse crashed the process. It now throws `.badURL`, which classifies as
    // `.nonExistentSiteError`. The real trigger is the strict parser on iOS 16 / macOS 13; an empty
    // string is rejected by every Foundation version, so it exercises the same path on any host.
    @Test("A URL Foundation can't parse is classified as .nonExistentSiteError instead of crashing")
    func testUnparseableURLIsClassifiedAsNonExistentSite() async throws {
        let executor = WpRequestExecutor(urlSession: .shared)

        let result = await executor.perform(UnparseableURLRequest())
        let reason = try #require(failureReason(result))

        guard case .nonExistentSiteError = reason else {
            Issue.record("Expected .nonExistentSiteError, got \(reason)")
            return
        }
    }

    // Regression test for #1515: a response that isn't an `HTTPURLResponse` hit a
    // `preconditionFailure` in `WpNetworkResponse.init`. It now throws `.badServerResponse`, which
    // classifies as `.httpError`.
    @Test("A non-HTTP response fails the request instead of crashing")
    func testNonHTTPResponseFailsTheRequest() async throws {
        let executor = WpRequestExecutor(urlSession: .shared)

        let result = await executor.perform(NonHTTPResponseRequest())
        let reason = try #require(failureReason(result))

        guard case .httpError = reason else {
            Issue.record("Expected .httpError, got \(reason)")
            return
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
    #if !os(Linux)
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
    // Excluded on watchOS: unlike a natural `.timedOut` (which round-trips fine there), watchOS's URL
    // loading system does not faithfully deliver these cellular-radio codes when they are *injected*
    // via `URLProtocolClient`, so the stub can't drive the branch on that platform. The classifier
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
    // Regression tests for #1505 and #1506: URLError codes with a closer classification than the
    // catch-all `.genericError`. Driven through the real URLSession completion path the same way as
    // the offline codes above, and excluded on watchOS for the same reason.
    @Test("An unsupported URL is classified as .nonExistentSiteError")
    func testUnsupportedURLIsClassifiedAsNonExistentSite() async throws {
        let reason = try await failureReason(forInjected: .unsupportedURL)

        guard case .nonExistentSiteError = reason else {
            Issue.record("Expected .nonExistentSiteError, got \(reason)")
            return
        }
    }

    @Test("An unanswerable authentication challenge is classified as .httpAuthenticationRequiredError")
    func testUserAuthenticationRequiredIsClassifiedAsHttpAuthenticationRequired() async throws {
        let reason = try await failureReason(forInjected: .userAuthenticationRequired)

        #expect(reason == .httpAuthenticationRequiredError(hostname: "example.com", method: nil))
    }

    @Test(
        "A client-certificate failure is classified as a generic SSL error",
        arguments: [URLError.Code.clientCertificateRequired, .clientCertificateRejected]
    )
    func testClientCertificateFailuresAreClassifiedAsGenericSslError(code: URLError.Code) async throws {
        let reason = try await failureReason(forInjected: code)

        #expect(reason == .invalidSslError(reason: .genericSslError))
    }

    // Regression tests for #1502 and #1503: the Swift executor never produced `.httpError`, so a
    // failed HTTP exchange — a malformed or undecodable response, or a redirect loop — fell through
    // to `.genericError`, while reqwest and Kotlin report the same class as `HttpError`.
    @Test(
        "HTTP-exchange failures are classified as .httpError",
        arguments: [
            URLError.Code.badServerResponse, .cannotParseResponse, .cannotDecodeRawData, .cannotDecodeContentData,
            .zeroByteResource, .dataLengthExceedsMaximum, .requestBodyStreamExhausted, .httpTooManyRedirects,
            .redirectToNonExistentLocation
        ]
    )
    func testHttpExchangeFailuresAreClassifiedAsHttpError(code: URLError.Code) async throws {
        let reason = try await failureReason(forInjected: code)

        guard case .httpError = reason else {
            Issue.record("Expected .httpError for \(code), got \(reason)")
            return
        }
    }

    // Regression test for #1520: the offline, timeout, cancelled, and generic branches hard-coded
    // `redirects: nil`, dropping the redirect trail the delegate had recorded. Redirect once, then
    // fail with each code, and assert the redirect is still attached.
    @Test(
        "A request that redirects and then fails keeps its redirect trail",
        arguments: [URLError.Code.notConnectedToInternet, .timedOut, .cancelled, .httpTooManyRedirects, .unknown]
    )
    func testRedirectsSurviveEveryFailureBranch(code: URLError.Code) async throws {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [RedirectThenFailURLProtocol.self]
        let session = URLSession(configuration: configuration)
        defer { session.finishTasksAndInvalidate() }
        let executor = WpRequestExecutor(urlSession: session)

        let result = await executor.perform(RedirectThenFailRequest(code: code))

        guard case .failure(.RequestExecutionFailed(_, let redirects, _, _, _)) = result else {
            Issue.record("Expected a RequestExecutionFailed failure for \(code), got \(result)")
            return
        }

        #expect(
            redirects == [
                WpRedirect(
                    source: RedirectThenFailRequest.source.absoluteString,
                    destination: RedirectThenFailRequest.destination.absoluteString
                )
            ]
        )
    }

    private func failureReason(forInjected code: URLError.Code) async throws -> RequestExecutionErrorReason {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [FailingURLProtocol.self]
        let session = URLSession(configuration: configuration)
        defer { session.finishTasksAndInvalidate() }
        let executor = WpRequestExecutor(urlSession: session)

        let result = await executor.perform(FailingRequest(code: code))
        return try #require(failureReason(result))
    }
    #endif // !os(watchOS)
    #endif

    /// The reason a request failed with, or `nil` if it didn't fail with `RequestExecutionFailed`.
    private func failureReason(
        _ result: Result<WpNetworkResponse, RequestExecutionError>
    ) -> RequestExecutionErrorReason? {
        guard case .failure(.RequestExecutionFailed(_, _, let reason, _, _)) = result else {
            return nil
        }
        return reason
    }
}

/// A `NetworkRequestContent` whose URL no version of Foundation can parse, so the executor's own
/// `buildURLRequest` is what fails.
private struct UnparseableURLRequest: NetworkRequestContent {
    func requestId() -> String { "1511-bad-url-regression" }
    func method() -> RequestMethod { .get }
    func url() -> WpEndpointUrl { "" }
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

/// A `NetworkRequestContent` that "completes" with a plain `URLResponse`, which URLSession never
/// produces for an http(s) load but `WpNetworkResponse.init` must still survive.
private struct NonHTTPResponseRequest: NetworkRequestContent {
    func requestId() -> String { "1515-non-http-response-regression" }
    func method() -> RequestMethod { .get }
    func url() -> WpEndpointUrl { "https://example.com/wp-json/" }
    func headerMap() -> WpNetworkHeaderMap { .empty }
    func encodeBody(into _: inout URLRequest) throws {}

    func perform(
        in _: URLSession,
        withAdditionalHeaders _: [String: String],
        delegate _: URLSessionTaskDelegate?
    ) async throws -> (Data, URLResponse) {
        let response = URLResponse(
            url: URL(string: url())!,
            mimeType: nil,
            expectedContentLength: 0,
            textEncodingName: nil
        )
        return (Data(), response)
    }
}

#if !os(Linux)
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

// Only used by the watchOS-excluded test above, so gated identically to avoid an unused-type warning.
#if !os(watchOS)
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
/// A `URLProtocol` that redirects the first request to `RedirectThenFailRequest.destination`, then
/// fails the redirected request with the `URLError.Code` carried in a request header — so a test can
/// assert the recorded redirect trail survives whichever failure branch the code lands in.
private final class RedirectThenFailURLProtocol: URLProtocol {
    override static func canInit(with request: URLRequest) -> Bool { true }
    override static func canonicalRequest(for request: URLRequest) -> URLRequest { request }
    override func startLoading() {
        guard let url = request.url else { return }

        if url == RedirectThenFailRequest.source {
            let response = HTTPURLResponse(
                url: url,
                statusCode: 302,
                httpVersion: "HTTP/1.1",
                headerFields: ["Location": RedirectThenFailRequest.destination.absoluteString]
            )!
            var redirected = request
            redirected.url = RedirectThenFailRequest.destination
            client?.urlProtocol(self, wasRedirectedTo: redirected, redirectResponse: response)
            return
        }

        let rawCode = request.value(forHTTPHeaderField: FailingURLProtocol.codeHeader).flatMap { Int($0) }
        let code = rawCode.map { URLError.Code(rawValue: $0) } ?? .unknown
        client?.urlProtocol(self, didFailWithError: URLError(code))
    }
    override func stopLoading() {}
}

/// A `NetworkRequestContent` that goes through the executor's delegate — which is what records
/// redirects — and is failed by `RedirectThenFailURLProtocol` after one redirect.
private struct RedirectThenFailRequest: NetworkRequestContent {
    static let source = URL(string: "https://example.com/wp-json/")!
    static let destination = URL(string: "https://www.example.com/wp-json/")!

    let code: URLError.Code

    func requestId() -> String { "1520-redirects-regression-\(code.rawValue)" }
    func method() -> RequestMethod { .get }
    func url() -> WpEndpointUrl { Self.source.absoluteString }
    func headerMap() -> WpNetworkHeaderMap { .empty }
    func encodeBody(into _: inout URLRequest) throws {}

    func perform(
        in session: URLSession,
        withAdditionalHeaders headers: [String: String],
        delegate: URLSessionTaskDelegate?
    ) async throws -> (Data, URLResponse) {
        var request = try buildURLRequest(additionalHeaders: headers)
        request.setValue(String(code.rawValue), forHTTPHeaderField: FailingURLProtocol.codeHeader)
        return try await session.data(for: request, delegate: delegate)
    }
}
#endif // !os(watchOS)
#endif
