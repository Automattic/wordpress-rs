import Foundation
import Testing
@testable import WordPressAPI
@testable import WordPressAPIInternal

// `upload(...)` sets its own per-task delegate, which forwards every callback to the delegate it's
// given (in production, `RequestExecutorDelegate`, which in turn notifies the app's Pulse logger).
// A task delegate shadows the session's for every method it implements, so a callback the wrapper
// fails to forward is silently lost. These tests pin each one.
#if os(macOS) || os(iOS)
@Suite("Upload task delegate forwarding", .timeLimit(.minutes(1)))
struct UploadTaskDelegateForwardingTests {
    typealias Support = UploadTestSupport

    /// Both forms of the challenge and redirect callbacks: `RequestExecutorDelegate` implements
    /// the `async` forms, and the app's delegate may implement either.
    static let delegateKinds: [DelegateKind] = [.completionHandler, .async]

    enum DelegateKind: CustomTestStringConvertible, Sendable {
        case completionHandler
        case async

        func make() -> RecordingDelegate {
            switch self {
            case .completionHandler: CompletionHandlerRecordingDelegate()
            case .async: AsyncRecordingDelegate()
            }
        }

        var testDescription: String {
            switch self {
            case .completionHandler: "completion-handler delegate"
            case .async: "async delegate"
            }
        }
    }

    // MARK: - Each event is reported exactly once

    @Test("A normal upload reports each event once")
    func normalUploadReportsEachEventOnce() async throws {
        let server = try await LoopbackHTTPServer.accepting()
        defer { server.stop() }
        let recorder = RecordingDelegate()

        _ = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: recorder
        )

        let events = try await recorder.waitForCompletion()
        #expect(events.createdTasks == 1)
        #expect(events.responses == [201])
        #expect(events.receivedData == Data(#"{"accepted":true}"#.utf8))
        #expect(events.completionErrorCodes == [nil])
        #expect(events.completions.allSatisfy { $0 == nil })
        #expect(events.metrics == 1)
        #expect(events.bodyDataCallbacks > 0)
        #expect(events.totalBytesSent == Int64(Support.largeBodySize))
    }

    @Test("An early response is reported once, as a completed request")
    func earlyResponseReportsEachEventOnce() async throws {
        let server = try await LoopbackHTTPServer.rejectingEarly()
        defer { server.stop() }
        let recorder = RecordingDelegate()

        _ = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: recorder
        )

        // `URLSession` reports the cancellation that ended the upload after `upload(...)` returns.
        let events = try await recorder.waitForCompletion()
        #expect(events.createdTasks == 1)
        #expect(events.responses == [413])
        #expect(events.receivedData == Data(LoopbackHTTPServer.rejectionBody.utf8))
        #expect(events.completions.count == 1)
        #expect(events.completions.allSatisfy { $0 == nil })
        #expect(events.metrics == 1)
    }

    @Test("An early response followed by a broken connection is reported as a completed request")
    func earlyResponseThenConnectionLossReportsCompletion() async throws {
        let server = try await LoopbackHTTPServer.start { exchange in
            let head = "HTTP/1.1 413 Payload Too Large\r\nContent-Type: text/html\r\nTransfer-Encoding: chunked\r\n\r\n"
            exchange.send(head + "6\r\n<html>\r\n")
            exchange.after(.milliseconds(200)) { exchange.disconnect() }
        }
        defer { server.stop() }
        let recorder = RecordingDelegate()

        _ = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: recorder
        )

        let events = try await recorder.waitForCompletion()
        #expect(events.responses == [413])
        #expect(events.completions.count == 1)
        #expect(events.completions.allSatisfy { $0 == nil })
    }

    @Test("A timed-out upload reports its error once")
    func timeoutReportsErrorOnce() async throws {
        let server = try await LoopbackHTTPServer.neverResponding()
        defer { server.stop() }
        let recorder = RecordingDelegate()

        await #expect(throws: URLError.self) {
            try await upload(
                body: .onDisk(try Support.makeTempFile()),
                with: Support.request(to: server.url),
                session: Support.session(requestTimeout: 0.5),
                delegate: recorder
            )
        }

        let events = try await recorder.waitForCompletion()
        #expect(events.responses.isEmpty)
        #expect(events.completionErrorCodes == [.timedOut])
    }

    @Test("A refused connection reports its error once and throws it unchanged")
    func refusedConnectionReportsErrorOnce() async throws {
        let server = try await LoopbackHTTPServer.neverResponding()
        server.stop()
        let recorder = RecordingDelegate()

        await #expect(
            performing: {
                try await upload(
                    body: .onDisk(try Support.makeTempFile()),
                    with: Support.request(to: server.url),
                    session: Support.session(),
                    delegate: recorder
                )
            },
            throws: { ($0 as? URLError)?.code == .cannotConnectToHost }
        )

        let events = try await recorder.waitForCompletion()
        #expect(events.completionErrorCodes == [.cannotConnectToHost])
    }

    @Test("A delegate that isn't a data delegate still gets task events")
    func taskOnlyDelegateGetsTaskEvents() async throws {
        let server = try await LoopbackHTTPServer.rejectingEarly()
        defer { server.stop() }
        let delegate = TaskOnlyDelegate()

        let (_, response) = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: delegate
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 413)
        try await Task.sleep(for: .milliseconds(300))
        #expect(delegate.completions == 1)
    }

    // MARK: - The notifying delegate observes, but doesn't control, the response

    @Test(
        "The notifying delegate's response disposition is ignored",
        arguments: [URLSession.ResponseDisposition.cancel, .becomeDownload]
    )
    func responseDispositionIsIgnored(disposition: URLSession.ResponseDisposition) async throws {
        let server = try await LoopbackHTTPServer.accepting()
        defer { server.stop() }
        let recorder = RecordingDelegate()
        recorder.responseDisposition = disposition

        let (data, response) = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: recorder
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 201)
        #expect(String(bytes: data, encoding: .utf8) == #"{"accepted":true}"#)
    }

    // MARK: - Redirects

    @Test("A redirect is forwarded to the delegate and followed", arguments: delegateKinds)
    func redirectIsForwardedAndFollowed(kind: DelegateKind) async throws {
        let server = try await Self.redirectingServer()
        defer { server.stop() }
        let recorder = kind.make()

        let (data, response) = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: recorder
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 201)
        #expect(String(bytes: data, encoding: .utf8) == "final")
        #expect(recorder.snapshot.redirects.map(\.from) == ["/upload"])
        #expect(recorder.snapshot.redirects.map(\.to) == ["/final"])
        // A 307 re-sends the whole body to the new location.
        #expect(server.requests.map(\.path) == ["/upload", "/final"])
        #expect(server.requests.last?.contentLength == Support.largeBodySize)
    }

    @Test("A redirect the delegate refuses returns the redirect response", arguments: delegateKinds)
    func refusedRedirectReturnsRedirectResponse(kind: DelegateKind) async throws {
        let server = try await Self.redirectingServer()
        defer { server.stop() }
        let recorder = kind.make()
        recorder.followsRedirects = false

        let (_, response) = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: recorder
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 307)
        #expect(server.requests.map(\.path) == ["/upload"])
    }

    @Test("A redirect is followed with no delegate")
    func redirectIsFollowedWithoutDelegate() async throws {
        let server = try await Self.redirectingServer()
        defer { server.stop() }

        let (_, response) = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: nil
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 201)
    }

    @Test("A redirect is followed with a delegate that doesn't handle redirects")
    func redirectIsFollowedWithTaskOnlyDelegate() async throws {
        let server = try await Self.redirectingServer()
        defer { server.stop() }

        let (_, response) = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: TaskOnlyDelegate()
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 201)
    }

    // MARK: - Authentication challenges

    @Test("An authentication challenge is forwarded to the delegate", arguments: delegateKinds)
    func challengeIsForwarded(kind: DelegateKind) async throws {
        let server = try await Self.basicAuthServer()
        defer { server.stop() }
        let recorder = kind.make()
        recorder.credential = URLCredential(user: "demo", password: "secret", persistence: .none)

        let (data, response) = try await upload(
            body: .inMemory(Data(count: 64 * 1024)),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: recorder
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 201)
        #expect(String(bytes: data, encoding: .utf8) == "authorized")
        #expect(recorder.snapshot.challenges == [NSURLAuthenticationMethodHTTPBasic])
        #expect(server.requests.last?.headers["authorization"] == "Basic ZGVtbzpzZWNyZXQ=")
    }

    @Test("A challenge the delegate declines returns the 401", arguments: delegateKinds)
    func declinedChallengeReturns401(kind: DelegateKind) async throws {
        let server = try await Self.basicAuthServer()
        defer { server.stop() }
        let recorder = kind.make()

        let (_, response) = try await upload(
            body: .inMemory(Data(count: 64 * 1024)),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: recorder
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 401)
        #expect(recorder.snapshot.challenges == [NSURLAuthenticationMethodHTTPBasic])
    }

    @Test("A challenge gets default handling with no delegate")
    func challengeWithoutDelegateReturns401() async throws {
        let server = try await Self.basicAuthServer()
        defer { server.stop() }

        let (_, response) = try await upload(
            body: .inMemory(Data(count: 64 * 1024)),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: nil
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 401)
    }

    // MARK: - Informational responses

    @Test("An informational response is forwarded to the delegate")
    func informationalResponseIsForwarded() async throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { return }
        let server = try await LoopbackHTTPServer.start { exchange in
            exchange.readBody { _ in
                exchange.send("HTTP/1.1 103 Early Hints\r\nLink: </style.css>; rel=preload\r\n\r\n")
                exchange.respond(status: 201, body: "done")
            }
        }
        defer { server.stop() }
        let recorder = RecordingDelegate()

        let (_, response) = try await upload(
            body: .inMemory(Data(count: 64 * 1024)),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: recorder
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 201)
        #expect(recorder.snapshot.informationalResponses == [103])
    }

    // MARK: - Helpers

    /// Answers `/upload` with a `307` to `/final` after reading the body, and `/final` with a `201`.
    private static func redirectingServer() async throws -> LoopbackHTTPServer {
        try await LoopbackHTTPServer.start { exchange in
            exchange.readBody { _ in
                if exchange.request.path == "/final" {
                    exchange.respond(status: 201, body: "final")
                } else {
                    exchange.respond(status: 307, headers: ["Location": "/final"])
                }
            }
        }
    }

    /// Answers with a Basic `401` until a request carries credentials, then a `201`.
    private static func basicAuthServer() async throws -> LoopbackHTTPServer {
        try await LoopbackHTTPServer.start { exchange in
            exchange.readBody { _ in
                if exchange.request.headers["authorization"] != nil {
                    exchange.respond(status: 201, body: "authorized")
                } else {
                    exchange.respond(status: 401, headers: ["WWW-Authenticate": #"Basic realm="test""#])
                }
            }
        }
    }
}

/// A delegate that implements only one task-level callback, and none of the data-delegate ones.
private final class TaskOnlyDelegate: NSObject, URLSessionTaskDelegate, @unchecked Sendable {
    private let lock = NSLock()
    private var _completions = 0
    var completions: Int { lock.withLock { _completions } }

    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: (any Error)?) {
        lock.withLock { _completions += 1 }
    }
}
#endif
