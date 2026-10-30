import Foundation

#if os(macOS) || os(iOS)
import Network

/// A minimal HTTP/1.1 server on `127.0.0.1`, for driving `URLSession` uploads through real sockets.
///
/// Each request is passed to the test's handler as an `Exchange`, which decides whether to read the
/// body, how to respond, and whether to keep or drop the connection. A `URLProtocol` stub can't stand
/// in for this: `URLSession` doesn't deliver a stub's response until the stub finishes loading, so a
/// stub can't answer before the body is sent.
final class LoopbackHTTPServer: @unchecked Sendable {
    struct Request: Sendable {
        let method: String
        let path: String
        /// Header names are lowercased.
        let headers: [String: String]

        var contentLength: Int { headers["content-length"].flatMap(Int.init) ?? 0 }
    }

    /// One request on one connection.
    final class Exchange: @unchecked Sendable {
        let request: Request
        private let connection: NWConnection
        private let queue: DispatchQueue
        private let onBodyRead: @Sendable (Int) -> Void
        private var pendingBody: Int

        fileprivate init(
            request: Request,
            connection: NWConnection,
            queue: DispatchQueue,
            bodyBytesAlreadyRead: Int,
            onBodyRead: @escaping @Sendable (Int) -> Void
        ) {
            self.request = request
            self.connection = connection
            self.queue = queue
            self.onBodyRead = onBodyRead
            self.pendingBody = request.contentLength - bodyBytesAlreadyRead
            onBodyRead(bodyBytesAlreadyRead)
        }

        /// Reads the rest of the request body, then calls `completion` with the number of body bytes read.
        ///
        /// With `pacing`, reads at most 256 KB at a time and waits that long between reads, to make
        /// an upload take a while.
        func readBody(pacing: DispatchTimeInterval? = nil, then completion: @escaping @Sendable (Int) -> Void) {
            guard pendingBody > 0 else {
                completion(request.contentLength)
                return
            }
            let maximumLength = pacing == nil ? 1024 * 1024 : 256 * 1024
            connection.receive(minimumIncompleteLength: 1, maximumLength: maximumLength) { content, _, _, error in
                guard error == nil, let content, !content.isEmpty else { return }
                self.pendingBody -= content.count
                self.onBodyRead(content.count)
                if let pacing {
                    self.after(pacing) { self.readBody(pacing: pacing, then: completion) }
                } else {
                    self.readBody(then: completion)
                }
            }
        }

        /// Sends raw bytes.
        func send(_ text: String, then completion: (@Sendable () -> Void)? = nil) {
            connection.send(content: Data(text.utf8), completion: .contentProcessed { _ in completion?() })
        }

        /// Sends a complete response. `closing` closes the connection gracefully once it's sent, as a
        /// server does after reading the whole body; leave it open to mimic a proxy that stops reading.
        func respond(
            status: Int,
            reason: String = "Status",
            headers: [String: String] = [:],
            body: String = "",
            includeContentLength: Bool = true,
            closing: Bool = true,
            then completion: (@Sendable () -> Void)? = nil
        ) {
            var headers = headers
            if headers["Content-Type"] == nil {
                headers["Content-Type"] = "text/html"
            }
            if includeContentLength {
                headers["Content-Length"] = String(body.utf8.count)
            }
            if closing {
                headers["Connection"] = "close"
            }
            let head = headers.map { "\($0.key): \($0.value)\r\n" }.sorted().joined()
            send("HTTP/1.1 \(status) \(reason)\r\n\(head)\r\n\(body)") { [connection] in
                if closing { connection.cancel() }
                completion?()
            }
        }

        /// Drops the connection.
        func disconnect() {
            connection.forceCancel()
        }

        func after(_ delay: DispatchTimeInterval, _ work: @escaping @Sendable () -> Void) {
            queue.asyncAfter(deadline: .now() + delay, execute: work)
        }
    }

    let url: URL
    private let listener: NWListener
    private let handler: @Sendable (Exchange) -> Void
    private let queue = DispatchQueue(label: "LoopbackHTTPServer")
    private let lock = NSLock()
    private var connections: [NWConnection] = []
    private var _requests: [Request] = []
    private var _bodyBytesRead = 0

    /// Every request received so far, in order.
    var requests: [Request] { lock.withLock { _requests } }

    /// Request body bytes read across all requests.
    var bodyBytesRead: Int { lock.withLock { _bodyBytesRead } }

    private init(listener: NWListener, port: NWEndpoint.Port, handler: @escaping @Sendable (Exchange) -> Void) {
        self.listener = listener
        self.handler = handler
        self.url = URL(string: "http://127.0.0.1:\(port.rawValue)")!
    }

    static func start(handler: @escaping @Sendable (Exchange) -> Void) async throws -> LoopbackHTTPServer {
        let parameters = NWParameters.tcp
        parameters.requiredLocalEndpoint = .hostPort(host: .ipv4(.loopback), port: .any)
        let listener = try NWListener(using: parameters)

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
            listener.newConnectionHandler = { $0.cancel() }
            listener.start(queue: DispatchQueue(label: "LoopbackHTTPServer.listener"))
        }

        let server = LoopbackHTTPServer(listener: listener, port: port, handler: handler)
        listener.newConnectionHandler = { [weak server] connection in server?.accept(connection) }
        return server
    }

    func stop() {
        listener.cancel()
        let connections = lock.withLock {
            defer { self.connections.removeAll() }
            return self.connections
        }
        connections.forEach { $0.cancel() }
    }

    /// Waits until `count` requests have arrived.
    func waitForRequests(_ count: Int = 1) async throws {
        let deadline = ContinuousClock.now + .seconds(10)
        while requests.count < count {
            guard ContinuousClock.now < deadline else {
                throw URLError(.timedOut)
            }
            try await Task.sleep(for: .milliseconds(10))
        }
    }

    private func accept(_ connection: NWConnection) {
        lock.withLock { connections.append(connection) }
        connection.start(queue: queue)
        readHeaders(from: connection, buffer: Data())
    }

    private func readHeaders(from connection: NWConnection, buffer: Data) {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 64 * 1024) { content, _, isComplete, error in
            var buffer = buffer
            if let content { buffer.append(content) }

            guard let headerEnd = buffer.range(of: Data("\r\n\r\n".utf8)) else {
                if error == nil && !isComplete { self.readHeaders(from: connection, buffer: buffer) }
                return
            }

            let lines = (String(bytes: buffer[..<headerEnd.lowerBound], encoding: .utf8) ?? "")
                .components(separatedBy: "\r\n")
            let requestLine = lines.first?.split(separator: " ") ?? []
            var headers: [String: String] = [:]
            for line in lines.dropFirst() {
                guard let colon = line.firstIndex(of: ":") else { continue }
                let name = line[..<colon].lowercased()
                headers[name] = line[line.index(after: colon)...].trimmingCharacters(in: .whitespaces)
            }
            let request = Request(
                method: requestLine.first.map(String.init) ?? "",
                path: requestLine.dropFirst().first.map(String.init) ?? "",
                headers: headers
            )
            self.lock.withLock { self._requests.append(request) }

            let exchange = Exchange(
                request: request,
                connection: connection,
                queue: self.queue,
                bodyBytesAlreadyRead: buffer.count - headerEnd.upperBound,
                onBodyRead: { [weak self] count in self?.lock.withLock { self?._bodyBytesRead += count } }
            )
            self.handler(exchange)
        }
    }
}

extension LoopbackHTTPServer {
    static let rejectionBody = "<html><body>413 Request Entity Too Large</body></html>"

    /// Answers with a `413` as soon as the headers arrive, never reads the body, and keeps the
    /// socket open — what Cloudflare does with an oversized upload. Like nginx, it says
    /// `Connection: close`, unless `keepAlive`, so `URLSession` won't reuse the connection.
    static func rejectingEarly(keepAlive: Bool = false) async throws -> LoopbackHTTPServer {
        try await start {
            $0.respond(
                status: 413,
                headers: keepAlive ? [:] : ["Connection": "close"],
                body: rejectionBody,
                closing: false
            )
        }
    }

    /// Reads the whole body, then answers with `status` and `body`.
    static func accepting(status: Int = 201, body: String = #"{"accepted":true}"#) async throws -> LoopbackHTTPServer {
        try await start { exchange in
            exchange.readBody { _ in
                exchange.respond(status: status, headers: ["Content-Type": "application/json"], body: body)
            }
        }
    }

    /// Reads the headers and never responds or reads the body.
    static func neverResponding() async throws -> LoopbackHTTPServer {
        try await start { _ in }
    }
}

/// A notifying delegate (Pulse, in the apps) that records every callback it receives.
///
/// Implements the completion-handler forms of the challenge and redirect callbacks; see
/// `AsyncRecordingDelegate` for the `async` forms, which `RequestExecutorDelegate` uses.
class RecordingDelegate: NSObject, URLSessionDataDelegate, @unchecked Sendable {
    struct Events {
        var createdTasks = 0
        var responses: [Int] = []
        var receivedData = Data()
        var dataCallbacks = 0
        var totalBytesSent: Int64 = 0
        var bodyDataCallbacks = 0
        var completions: [Error?] = []
        var metrics = 0
        var redirects: [(from: String, to: String)] = []
        var challenges: [String] = []
        var informationalResponses: [Int] = []
        /// `networkProtocolName` of each transaction, e.g. `h3`.
        var protocols: [String?] = []
        var bytesSentAtCompletion: Int64?

        var completionErrorCodes: [URLError.Code?] { completions.map { ($0 as? URLError)?.code } }
    }

    let lock = NSLock()
    var events = Events()

    /// Whether to follow redirects.
    var followsRedirects = true
    /// The credential to answer HTTP authentication challenges with, if any.
    var credential: URLCredential?
    /// The disposition to answer `didReceive response` with. `UploadTaskDelegate` must ignore it.
    var responseDisposition: URLSession.ResponseDisposition = .allow

    var snapshot: Events { lock.withLock { events } }

    /// Waits for `didCompleteWithError`, which can arrive after `upload(...)` returns, then a little
    /// longer so a duplicate call would be caught.
    func waitForCompletion() async throws -> Events {
        let deadline = ContinuousClock.now + .seconds(10)
        while lock.withLock({ events.completions.isEmpty }) && ContinuousClock.now < deadline {
            try await Task.sleep(for: .milliseconds(20))
        }
        try await Task.sleep(for: .milliseconds(200))
        return snapshot
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
        lock.withLock {
            events.bodyDataCallbacks += 1
            events.totalBytesSent = max(events.totalBytesSent, totalBytesSent)
        }
    }

    func urlSession(
        _ session: URLSession,
        dataTask: URLSessionDataTask,
        didReceive response: URLResponse,
        completionHandler: @escaping @Sendable (URLSession.ResponseDisposition) -> Void
    ) {
        lock.withLock { events.responses.append((response as? HTTPURLResponse)?.statusCode ?? -1) }
        completionHandler(responseDisposition)
    }

    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask, didReceive data: Data) {
        lock.withLock {
            events.receivedData.append(data)
            events.dataCallbacks += 1
        }
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didFinishCollecting metrics: URLSessionTaskMetrics) {
        lock.withLock {
            events.metrics += 1
            events.protocols.append(contentsOf: metrics.transactionMetrics.map(\.networkProtocolName))
        }
    }

    func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        didReceiveInformationalResponse response: HTTPURLResponse
    ) {
        lock.withLock { events.informationalResponses.append(response.statusCode) }
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: (any Error)?) {
        lock.withLock {
            events.completions.append(error)
            events.bytesSentAtCompletion = task.countOfBytesSent
        }
    }

    func recordRedirect(_ response: HTTPURLResponse, to request: URLRequest) -> URLRequest? {
        lock.withLock {
            events.redirects.append((response.url?.path ?? "", request.url?.path ?? ""))
        }
        return followsRedirects ? request : nil
    }

    func recordChallenge(
        _ challenge: URLAuthenticationChallenge
    ) -> (URLSession.AuthChallengeDisposition, URLCredential?) {
        lock.withLock { events.challenges.append(challenge.protectionSpace.authenticationMethod) }
        guard challenge.protectionSpace.authenticationMethod == NSURLAuthenticationMethodHTTPBasic,
            let credential,
            challenge.previousFailureCount == 0
        else {
            return (.performDefaultHandling, nil)
        }
        return (.useCredential, credential)
    }
}

/// A `RecordingDelegate` with the completion-handler forms of the challenge and redirect callbacks.
final class CompletionHandlerRecordingDelegate: RecordingDelegate, @unchecked Sendable {
    func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        willPerformHTTPRedirection response: HTTPURLResponse,
        newRequest request: URLRequest,
        completionHandler: @escaping @Sendable (URLRequest?) -> Void
    ) {
        completionHandler(recordRedirect(response, to: request))
    }

    func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        didReceive challenge: URLAuthenticationChallenge,
        completionHandler: @escaping @Sendable (URLSession.AuthChallengeDisposition, URLCredential?) -> Void
    ) {
        let (disposition, credential) = recordChallenge(challenge)
        completionHandler(disposition, credential)
    }
}

/// A `RecordingDelegate` with the `async` forms of the challenge and redirect callbacks, as
/// `RequestExecutorDelegate` implements them.
final class AsyncRecordingDelegate: RecordingDelegate, @unchecked Sendable {
    func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        willPerformHTTPRedirection response: HTTPURLResponse,
        newRequest request: URLRequest
    ) async -> URLRequest? {
        recordRedirect(response, to: request)
    }

    func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        didReceive challenge: URLAuthenticationChallenge
    ) async -> (URLSession.AuthChallengeDisposition, URLCredential?) {
        recordChallenge(challenge)
    }
}

/// Helpers shared by the upload test suites.
enum UploadTestSupport {
    /// Much more than the loopback socket buffers hold, so a server that doesn't read the body
    /// leaves most of it unsent — as Cloudflare does.
    static let largeBodySize = 32 * 1024 * 1024

    /// Long enough that a test returning before it proves the early response was used.
    static let requestTimeout: TimeInterval = 10

    /// A file of `size` zero bytes. It's sparse — sized without writing anything — because dozens
    /// of tests making 32 MB files at once tie up Swift's concurrency threads, which delays every
    /// other test that sleeps or awaits.
    static func makeTempFile(size: Int = largeBodySize) throws -> URL {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try Data().write(to: url)
        let handle = try FileHandle(forWritingTo: url)
        defer { try? handle.close() }
        try handle.truncate(atOffset: UInt64(size))
        return url
    }

    static func session(requestTimeout: TimeInterval = requestTimeout) -> URLSession {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = requestTimeout
        return URLSession(configuration: configuration)
    }

    static func request(to url: URL, path: String = "/upload") -> URLRequest {
        var request = URLRequest(url: url.appendingPathComponent(path))
        request.httpMethod = "POST"
        return request
    }
}
#endif
