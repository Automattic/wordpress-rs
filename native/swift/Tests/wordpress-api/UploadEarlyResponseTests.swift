import Foundation
import Testing
@testable import WordPressAPI
@testable import WordPressAPIInternal

#if canImport(Network)
import Network
#endif

// A server can answer an upload before it has read the whole body — Cloudflare rejects an oversized
// upload with a `413` after about 1–2 MB (CMM-2500). `URLSession` receives that response, but a
// completion-handler task never completes: it waits to send the rest of the body and then times out,
// so the app reported `httpTimeoutError` instead of the `413`.
//
// A `URLProtocol` stub can't reproduce this, because `URLSession` doesn't deliver a stub's response
// until the stub finishes loading. These tests use a real HTTP/1.1 server on the loopback interface
// that answers without reading the body and keeps the connection open, which hangs `URLSession` the
// same way Cloudflare does over HTTP/3.
//
// Gated to macOS and iOS, like the other `upload(...)` tests in `MultipartFormTests`.
#if os(macOS) || os(iOS)
@Suite("Upload early responses", .timeLimit(.minutes(1)))
struct UploadEarlyResponseTests {

    /// Much more than the loopback socket buffers hold, so a server that doesn't read the body
    /// leaves most of it unsent — as Cloudflare does.
    static let bodySize = 32 * 1024 * 1024

    /// Long enough that a test returning before it proves the early response was used.
    static let requestTimeout: TimeInterval = 10

    @Test("An early 413 is returned without waiting for the request timeout")
    func earlyResponseWithContentLengthIsReturned() async throws {
        let server = try await LoopbackHTTPServer.start(behavior: .rejectWithoutReading(sendContentLength: true))
        defer { server.stop() }
        let file = try Self.makeTempFile()

        let clock = ContinuousClock()
        let start = clock.now
        let (data, response) = try await upload(
            body: .onDisk(file),
            with: Self.request(to: server.url),
            session: Self.session(),
            delegate: nil
        )
        let elapsed = start.duration(to: clock.now)

        #expect((response as? HTTPURLResponse)?.statusCode == 413)
        #expect(String(data: data, encoding: .utf8) == LoopbackHTTPServer.rejectionBody)
        // Well under the quiet period too: the `Content-Length` says the body is complete.
        #expect(elapsed < .milliseconds(900))
        #expect(!FileManager.default.fileExists(atPath: file.path))
    }

    @Test("An early response without a Content-Length is returned after the quiet period")
    func earlyResponseWithoutContentLengthIsReturned() async throws {
        let server = try await LoopbackHTTPServer.start(behavior: .rejectWithoutReading(sendContentLength: false))
        defer { server.stop() }

        let clock = ContinuousClock()
        let start = clock.now
        let (data, response) = try await upload(
            body: .onDisk(try Self.makeTempFile()),
            with: Self.request(to: server.url),
            session: Self.session(),
            delegate: nil
        )
        let elapsed = start.duration(to: clock.now)

        #expect((response as? HTTPURLResponse)?.statusCode == 413)
        #expect(String(data: data, encoding: .utf8) == LoopbackHTTPServer.rejectionBody)
        #expect(elapsed >= .milliseconds(900))
        #expect(elapsed < .seconds(5))
    }

    @Test("An early response is returned even if the connection then fails")
    func earlyResponseIsPreferredOverLaterError() async throws {
        let server = try await LoopbackHTTPServer.start(behavior: .rejectWithoutReadingThenDisconnect)
        defer { server.stop() }

        let (data, response) = try await upload(
            body: .onDisk(try Self.makeTempFile()),
            with: Self.request(to: server.url),
            session: Self.session(),
            delegate: nil
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 413)
        // The server promised more bytes than it sent, so only what arrived is returned.
        #expect(String(data: data, encoding: .utf8) == LoopbackHTTPServer.partialRejectionBody)
    }

    @Test("A response after the full body is returned unchanged, and every event is reported once")
    func normalResponseIsUnchanged() async throws {
        let server = try await LoopbackHTTPServer.start(behavior: .acceptBody)
        defer { server.stop() }
        let file = try Self.makeTempFile()
        let recorder = RecordingDelegate()

        let (data, response) = try await upload(
            body: .onDisk(file),
            with: Self.request(to: server.url),
            session: Self.session(),
            delegate: recorder
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 201)
        #expect(String(data: data, encoding: .utf8) == LoopbackHTTPServer.acceptanceBody)
        #expect(!FileManager.default.fileExists(atPath: file.path))

        let events = try await recorder.waitForCompletion()
        #expect(events.createdTasks == 1)
        #expect(events.responses == [201])
        #expect(events.receivedData == Data(LoopbackHTTPServer.acceptanceBody.utf8))
        #expect(events.completions.count == 1)
        #expect(events.completions.first.map { $0 == nil } == true)
        // Upload progress (what Pulse shows) still reaches the app's delegate.
        #expect(events.totalBytesSent == Int64(Self.bodySize))
    }

    @Test("An early response is reported to the app's delegate once, as a completed request")
    func earlyResponseIsReportedOnce() async throws {
        let server = try await LoopbackHTTPServer.start(behavior: .rejectWithoutReading(sendContentLength: true))
        defer { server.stop() }
        let recorder = RecordingDelegate()

        _ = try await upload(
            body: .onDisk(try Self.makeTempFile()),
            with: Self.request(to: server.url),
            session: Self.session(),
            delegate: recorder
        )

        // `URLSession` reports the cancellation that ended the upload after `upload(...)` returns.
        let events = try await recorder.waitForCompletion()
        #expect(events.responses == [413])
        #expect(events.receivedData == Data(LoopbackHTTPServer.rejectionBody.utf8))
        #expect(events.completions.count == 1)
        #expect(events.completions.first.map { $0 == nil } == true)
    }

    @Test("An early 413 surfaces end-to-end with the 'too large' message")
    func earlyResponseSurfacesEndToEnd() async throws {
        let server = try await LoopbackHTTPServer.start(behavior: .rejectWithoutReading(sendContentLength: true))
        defer { server.stop() }
        let file = try Self.makeTempFile()
        defer { try? FileManager.default.removeItem(at: file) }

        do {
            _ = try await Self.api(for: server).media.create(params: .init(filePath: file.path))
            Issue.record("Expected the upload to be rejected, but it succeeded")
        } catch let error as WpApiError {
            guard case .UnknownError(let statusCode, _, _, _) = error else {
                Issue.record("Expected .UnknownError, got \(error)")
                return
            }
            #expect(statusCode == 413)
            #expect(error.localizedDescription == "The file is too large for this site's server to accept.")
        }
    }

    @Test("An upload the server never answers still times out as .httpTimeoutError")
    func unansweredUploadTimesOut() async throws {
        let server = try await LoopbackHTTPServer.start(behavior: .neverRespond)
        defer { server.stop() }
        let file = try Self.makeTempFile()
        defer { try? FileManager.default.removeItem(at: file) }

        do {
            _ = try await Self.api(for: server, requestTimeout: 0.5).media.create(params: .init(filePath: file.path))
            Issue.record("Expected the upload to time out, but it succeeded")
        } catch {
            let reason = (error as? CarriesRequestExecutionErrorReason)?.executionErrorReason
            guard case .some(.httpTimeoutError) = reason else {
                Issue.record("Expected .httpTimeoutError, got \(String(describing: reason)) (error: \(error))")
                return
            }
        }
    }

    @Test("Upload progress still reaches the Progress passed to uploadMedia")
    func uploadMediaProgressIsReported() async throws {
        let server = try await LoopbackHTTPServer.start(behavior: .acceptBody)
        defer { server.stop() }
        let file = try Self.makeTempFile()
        defer { try? FileManager.default.removeItem(at: file) }
        let progress = Progress()

        // The stub's body isn't a media object, so the call fails to parse; only progress matters.
        _ = try? await Self.api(for: server).uploadMedia(params: .init(filePath: file.path), fulfilling: progress)

        #expect(progress.fractionCompleted > 0.99)
    }

    // MARK: - Helpers

    private static func makeTempFile() throws -> URL {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try Data(count: bodySize).write(to: url)
        return url
    }

    private static func session(requestTimeout: TimeInterval = requestTimeout) -> URLSession {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = requestTimeout
        return URLSession(configuration: configuration)
    }

    private static func request(to url: URL) -> URLRequest {
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        return request
    }

    private static func api(
        for server: LoopbackHTTPServer,
        requestTimeout: TimeInterval = requestTimeout
    ) throws -> WordPressAPI {
        try WordPressAPI(
            siteInfo: .selfHosted(
                siteUrl: ParsedUrl.parse(input: server.url.absoluteString),
                apiRoot: ParsedUrl.parse(input: server.url.appendingPathComponent("wp-json").absoluteString)
            ),
            authenticationProvider: .none(),
            executor: WpRequestExecutor(urlSession: session(requestTimeout: requestTimeout)),
            middlewarePipeline: .default,
            appNotifier: nil
        )
    }
}

/// A minimal HTTP/1.1 server on `127.0.0.1` that answers every request the same way.
private final class LoopbackHTTPServer: @unchecked Sendable {
    enum Behavior {
        /// Responds with a `413` as soon as the request headers arrive, never reads the body, and
        /// keeps the connection open.
        case rejectWithoutReading(sendContentLength: Bool)
        /// Like `rejectWithoutReading`, but sends only part of a chunked response body and then
        /// drops the connection, so the task fails.
        case rejectWithoutReadingThenDisconnect
        /// Reads the whole body, then responds with a `201`.
        case acceptBody
        /// Reads the request headers and never responds.
        case neverRespond
    }

    static let rejectionBody = "<html><body>413 Request Entity Too Large</body></html>"
    static let partialRejectionBody = "<html>"
    static let acceptanceBody = #"{"accepted":true}"#

    let url: URL
    private let behavior: Behavior
    private let listener: NWListener
    private let queue = DispatchQueue(label: "LoopbackHTTPServer")
    private var connections: [NWConnection] = []

    private init(behavior: Behavior, listener: NWListener, port: NWEndpoint.Port) {
        self.behavior = behavior
        self.listener = listener
        self.url = URL(string: "http://127.0.0.1:\(port.rawValue)")!
    }

    static func start(behavior: Behavior) async throws -> LoopbackHTTPServer {
        let parameters = NWParameters.tcp
        parameters.requiredLocalEndpoint = .hostPort(host: .ipv4(.loopback), port: .any)
        let listener = try NWListener(using: parameters)
        let queue = DispatchQueue(label: "LoopbackHTTPServer.start")

        let port: NWEndpoint.Port = try await withCheckedThrowingContinuation { continuation in
            listener.stateUpdateHandler = { state in
                switch state {
                case .ready:
                    listener.stateUpdateHandler = nil
                    continuation.resume(returning: listener.port!)
                case .failed(let error):
                    listener.stateUpdateHandler = nil
                    continuation.resume(throwing: error)
                default:
                    break
                }
            }
            listener.newConnectionHandler = { _ in }
            listener.start(queue: queue)
        }

        let server = LoopbackHTTPServer(behavior: behavior, listener: listener, port: port)
        listener.newConnectionHandler = { [weak server] connection in server?.accept(connection) }
        return server
    }

    func stop() {
        listener.cancel()
        queue.sync {
            connections.forEach { $0.cancel() }
            connections.removeAll()
        }
    }

    private func accept(_ connection: NWConnection) {
        queue.async {
            self.connections.append(connection)
            connection.start(queue: self.queue)
            self.readHeaders(from: connection, buffer: Data())
        }
    }

    private func readHeaders(from connection: NWConnection, buffer: Data) {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 64 * 1024) { content, _, isComplete, error in
            var buffer = buffer
            if let content { buffer.append(content) }

            guard let headerEnd = buffer.range(of: Data("\r\n\r\n".utf8)) else {
                if error == nil && !isComplete { self.readHeaders(from: connection, buffer: buffer) }
                return
            }

            let headers = String(bytes: buffer[..<headerEnd.lowerBound], encoding: .utf8) ?? ""
            let bodyBytesRead = buffer.count - headerEnd.upperBound
            self.respond(on: connection, headers: headers, bodyBytesRead: bodyBytesRead)
        }
    }

    private func respond(on connection: NWConnection, headers: String, bodyBytesRead: Int) {
        switch behavior {
        case .rejectWithoutReading(let sendContentLength):
            let lengthHeader = sendContentLength ? "Content-Length: \(Self.rejectionBody.utf8.count)\r\n" : ""
            // A `Content-Type`, as Cloudflare and nginx send: without one, `URLSession` holds back a
            // response with no `Content-Length` to sniff its type from more of the body.
            let head = "HTTP/1.1 413 Payload Too Large\r\nContent-Type: text/html\r\n\(lengthHeader)\r\n"
            send(head + Self.rejectionBody, on: connection)
        case .rejectWithoutReadingThenDisconnect:
            // A chunked body cut off before its last chunk is an error; a short `Content-Length`
            // body isn't — `URLSession` reports that as a success.
            let partial = Self.partialRejectionBody
            let head = "HTTP/1.1 413 Payload Too Large\r\nContent-Type: text/html\r\nTransfer-Encoding: chunked\r\n\r\n"
            send(head + String(partial.utf8.count, radix: 16) + "\r\n" + partial + "\r\n", on: connection)
            queue.asyncAfter(deadline: .now() + .milliseconds(200)) { connection.forceCancel() }
        case .acceptBody:
            let contentLength =
                headers
                .split(separator: "\r\n")
                .first { $0.lowercased().hasPrefix("content-length:") }
                .flatMap { Int($0.split(separator: ":")[1].trimmingCharacters(in: .whitespaces)) } ?? 0
            readBody(from: connection, remaining: contentLength - bodyBytesRead)
        case .neverRespond:
            break
        }
    }

    private func readBody(from connection: NWConnection, remaining: Int) {
        guard remaining > 0 else {
            let body = Self.acceptanceBody
            let head =
                "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: \(body.utf8.count)\r\n\r\n"
            send(head + body, on: connection)
            return
        }
        connection.receive(minimumIncompleteLength: 1, maximumLength: 1024 * 1024) { content, _, isComplete, error in
            guard error == nil, !isComplete || content != nil else { return }
            self.readBody(from: connection, remaining: remaining - (content?.count ?? 0))
        }
    }

    private func send(_ response: String, on connection: NWConnection) {
        connection.send(content: Data(response.utf8), completion: .contentProcessed { _ in })
    }
}

/// Records the callbacks the app's notifying delegate (Pulse, in the apps) receives.
private final class RecordingDelegate: NSObject, URLSessionDataDelegate, @unchecked Sendable {
    struct Events {
        var createdTasks = 0
        var responses: [Int] = []
        var receivedData = Data()
        var totalBytesSent: Int64 = 0
        var completions: [Error?] = []
    }

    private let lock = NSLock()
    private var events = Events()

    /// Waits for `didCompleteWithError`, which can arrive after `upload(...)` returns, then a little
    /// longer so a duplicate call would be caught.
    func waitForCompletion() async throws -> Events {
        let clock = ContinuousClock()
        let deadline = clock.now + .seconds(5)
        while lock.withLock({ events.completions.isEmpty }) && clock.now < deadline {
            try await Task.sleep(for: .milliseconds(20))
        }
        try await Task.sleep(for: .milliseconds(200))
        return lock.withLock { events }
    }

    func urlSession(_ session: URLSession, didCreateTask task: URLSessionTask) {
        lock.withLock { events.createdTasks += 1 }
    }

    func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        didSendBodyData bytesSent: Int64,
        totalBytesSent: Int64,
        totalBytesExpectedToSend: Int64
    ) {
        lock.withLock { events.totalBytesSent = max(events.totalBytesSent, totalBytesSent) }
    }

    func urlSession(
        _ session: URLSession,
        dataTask: URLSessionDataTask,
        didReceive response: URLResponse,
        completionHandler: @escaping @Sendable (URLSession.ResponseDisposition) -> Void
    ) {
        lock.withLock { events.responses.append((response as? HTTPURLResponse)?.statusCode ?? -1) }
        completionHandler(.allow)
    }

    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask, didReceive data: Data) {
        lock.withLock { events.receivedData.append(data) }
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: (any Error)?) {
        lock.withLock { events.completions.append(error) }
    }
}
#endif
