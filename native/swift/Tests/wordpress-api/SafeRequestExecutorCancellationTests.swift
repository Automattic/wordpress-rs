import Foundation
import Testing
@testable import WordPressAPI
@testable import WordPressAPIInternal

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

// Compiled out on Linux: these drive requests through `URLProtocol` stubs, whose interaction with
// swift-corelibs-foundation's URLSession differs from Apple platforms.
#if !os(Linux)
@Suite("SafeRequestExecutor cancellation")
struct SafeRequestExecutorCancellationTests {

    // Regression test for #1517: `cancel(context:)` only reached a URLSession task that existed within
    // a one-second window, so cancelling during a `RetryAfterMiddleware` backoff — when no task is
    // alive — cancelled nothing: the retry went out and the call completed as if never cancelled.
    // The first request gets a 429 with `Retry-After: 2` — longer than that window — and the context is
    // cancelled during the backoff: the retry must fail with `.cancellationError` without ever
    // reaching the server.
    @Test("Cancelling a context during a Retry-After backoff cancels the retry", .timeLimit(.minutes(1)))
    func testCancellingDuringRetryBackoffCancelsTheRetry() async throws {
        let host = "retry-\(UUID().uuidString.lowercased()).example.com"
        let stub = try StubbedAPI(
            host: host,
            protocolClass: RateLimitedURLProtocol.self,
            middlewares: [RetryAfterMiddleware(maxRetries: 3, maxRetryWaitSeconds: 5)]
        )
        defer { stub.session.finishTasksAndInvalidate() }

        let context = RequestContext()
        let call = Task { [apiRoot = stub.api.apiRoot] in
            try await apiRoot.getCancellation(context: context)
        }

        // Wait for the 429 to be served, so the middleware is in its backoff with no task alive.
        try await waitUntil { StubState.requestCount(for: host) == 1 }
        try await Task.sleep(for: .milliseconds(100))
        stub.executor.cancel(context: context)

        let reason = await executionErrorReason(of: call)
        #expect(reason == .cancellationError)
        // The retry was cancelled before it was sent, not after the server answered it.
        #expect(StubState.requestCount(for: host) == 1)
    }

    // Companion to the test above for the in-flight case: `cancel(context:)` reaches a task that's
    // already running. This path used to look the task up through `session.allTasks` plus a Combine
    // publisher, and was compiled out entirely where Combine is unavailable (#1519).
    @Test("Cancelling a context cancels its in-flight request", .timeLimit(.minutes(1)))
    func testCancellingContextCancelsInFlightRequest() async throws {
        let host = "in-flight-\(UUID().uuidString.lowercased()).example.com"
        let stub = try StubbedAPI(host: host, protocolClass: HangingURLProtocol.self)
        defer { stub.session.finishTasksAndInvalidate() }

        let context = RequestContext()
        let call = Task { [apiRoot = stub.api.apiRoot] in
            try await apiRoot.getCancellation(context: context)
        }

        try await waitUntil { StubState.requestCount(for: host) == 1 }
        stub.executor.cancel(context: context)

        let reason = await executionErrorReason(of: call)
        #expect(reason == .cancellationError)
    }

    // Regression test for #1518: `withTaskCancellationHandler` runs `onCancel` immediately for an
    // already-cancelled Swift `Task` — before the URLSession task exists — and `TaskCancellation`
    // didn't remember it, so the request ran to completion. With the latch, the task is cancelled as
    // soon as it's assigned. Driven through `upload(body:with:session:delegate:)`, which shares
    // `TaskCancellation` with `WpNetworkRequest.perform`. Without the fix the hanging stub keeps the
    // request open until the session's timeout, so it fails with `.timedOut` rather than `.cancelled`.
    @Test("A request started from an already-cancelled Task is cancelled", .timeLimit(.minutes(1)))
    func testPreCancelledTaskCancelsTheRequest() async throws {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [HangingURLProtocol.self]
        configuration.timeoutIntervalForRequest = 5
        let session = URLSession(configuration: configuration)
        defer { session.finishTasksAndInvalidate() }

        let request = URLRequest(url: URL(string: "https://pre-cancelled.example.com/wp-json/")!)
        let call = Task {
            withUnsafeCurrentTask { $0?.cancel() }
            return try await upload(body: .inMemory(Data()), with: request, session: session, delegate: nil)
        }

        do {
            _ = try await call.value
            Issue.record("Expected the request to be cancelled, but it succeeded")
        } catch {
            #expect((error as? URLError)?.code == .cancelled, "got \(error)")
        }
    }

    private func executionErrorReason<T>(of call: Task<T, Error>) async -> RequestExecutionErrorReason? {
        do {
            _ = try await call.value
            Issue.record("Expected the call to fail, but it succeeded")
            return nil
        } catch {
            return (error as? CarriesRequestExecutionErrorReason)?.executionErrorReason
        }
    }

    private func waitUntil(_ condition: () -> Bool) async throws {
        while !condition() {
            try await Task.sleep(for: .milliseconds(10))
        }
    }
}

/// A `WordPressAPI` whose requests are all answered by `protocolClass`.
private struct StubbedAPI {
    let api: WordPressAPI
    let executor: WpRequestExecutor
    let session: URLSession

    init(host: String, protocolClass: URLProtocol.Type, middlewares: [Middleware] = []) throws {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [protocolClass]
        session = URLSession(configuration: configuration)
        executor = WpRequestExecutor(urlSession: session)
        api = try WordPressAPI(
            siteInfo: .selfHosted(
                siteUrl: ParsedUrl.parse(input: "https://\(host)"),
                apiRoot: ParsedUrl.parse(input: "https://\(host)/wp-json")
            ),
            authenticationProvider: .none(),
            executor: executor,
            middlewarePipeline: MiddlewarePipeline(middlewares: middlewares),
            appNotifier: nil
        )
    }
}

/// Requests seen by the stub protocols, keyed by host so concurrently running tests don't interfere.
private enum StubState {
    private static let lock = NSLock()
    nonisolated(unsafe) private static var counts: [String: Int] = [:]

    static func recordRequest(for host: String) -> Int {
        lock.withLock {
            counts[host, default: 0] += 1
            return counts[host]!
        }
    }

    static func requestCount(for host: String) -> Int {
        lock.withLock { counts[host, default: 0] }
    }
}

/// Answers the first request for a host with `429 Too Many Requests` and `Retry-After: 2`, and every
/// later one with `200 {}`.
private final class RateLimitedURLProtocol: URLProtocol {
    override static func canInit(with request: URLRequest) -> Bool { true }
    override static func canonicalRequest(for request: URLRequest) -> URLRequest { request }
    override func startLoading() {
        guard let url = request.url, let host = url.host else { return }

        let isFirst = StubState.recordRequest(for: host) == 1
        let response = HTTPURLResponse(
            url: url,
            statusCode: isFirst ? 429 : 200,
            httpVersion: "HTTP/1.1",
            headerFields: isFirst ? ["Retry-After": "2"] : ["Content-Type": "application/json"]
        )!
        client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
        client?.urlProtocol(self, didLoad: Data("{}".utf8))
        client?.urlProtocolDidFinishLoading(self)
    }
    override func stopLoading() {}
}

/// Records each request and then never responds, so it stays in flight until cancelled or timed out.
private final class HangingURLProtocol: URLProtocol {
    override static func canInit(with request: URLRequest) -> Bool { true }
    override static func canonicalRequest(for request: URLRequest) -> URLRequest { request }
    override func startLoading() {
        if let host = request.url?.host {
            _ = StubState.recordRequest(for: host)
        }
    }
    override func stopLoading() {}
}
#endif
