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
    #endif
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
#endif
