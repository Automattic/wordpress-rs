import Foundation
import WordPressAPIInternal

#if canImport(UniformTypeIdentifiers)
import UniformTypeIdentifiers
#endif

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

#if canImport(Combine)
import Combine
#endif

public protocol SafeRequestExecutor: RequestExecutor, Sendable {
    func execute(_ request: WpNetworkRequest) async -> Result<WpNetworkResponse, RequestExecutionError>
    func upload(request: WpMultipartFormRequest) async -> Result<WpNetworkResponse, RequestExecutionError>

    #if PROGRESS_REPORTING_ENABLED
    func progresses(for context: RequestContext) -> AnyPublisher<Progress, Never>
    #endif
}

extension SafeRequestExecutor {
    public func execute(request: WpNetworkRequest) async throws -> WpNetworkResponse {
        let result = await execute(request)
        return try result.get()
    }

    public func upload(request: WpMultipartFormRequest) async throws -> WpNetworkResponse {
        let result = await upload(request: request)
        return try result.get()
    }
}

public final class WpRequestExecutor: SafeRequestExecutor {
    private let session: URLSession
    private let executorDelegate: RequestExecutorDelegate

    private let additionalHttpHeadersForAllRequests: [String: String]

    public init(
        urlSession: URLSession,
        additionalHttpHeadersForAllRequests: [String: String] = [:],
        userAgent: String = defaultUserAgent(clientSpecificPostfix: UserAgent.postfix),
        notifyingDelegate: URLSessionTaskDelegate? = nil
    ) {
        self.session = urlSession
        self.executorDelegate = RequestExecutorDelegate(delegate: notifyingDelegate)

        var headers = additionalHttpHeadersForAllRequests
        if !headers.contains(where: { $0.key.caseInsensitiveCompare("User-Agent") == .orderedSame }) {
            headers["User-Agent"] = userAgent
        }
        self.additionalHttpHeadersForAllRequests = headers
    }

    public func execute(_ request: WpNetworkRequest) async -> Result<WpNetworkResponse, RequestExecutionError> {
        await perform(request)
    }

    public func upload(request: WpMultipartFormRequest) async -> Result<WpNetworkResponse, RequestExecutionError> {
        await perform(request)
    }

    public func cancel(context: RequestContext) {
        for requestId in context.requestIds() {
            Task {
                await self.cancelRequest(withId: requestId)
            }
        }
    }

    public func allowSSL(altNames: [String], forCommonName name: String) {
        executorDelegate.allowSSL(altNames: altNames, forCommonName: name)
    }

    func perform(_ request: NetworkRequestContent) async -> Result<WpNetworkResponse, RequestExecutionError> {
        do {
            let (data, response) = try await request.perform(
                in: session,
                withAdditionalHeaders: self.additionalHttpHeadersForAllRequests,
                delegate: executorDelegate
            )

            return .success(try WpNetworkResponse(data: data, request: request, response: response))
        } catch {
            if let error = error as? RequestExecutionError {
                return .failure(error)
            }

            if errorIsHttpsError(error) {
                return handleHttpsError(error, for: request)
            }

            if errorIsNonExistentSiteError(error) {
                return handleNonExistentSiteError(error, for: request)
            }

            if errorIsConnectionError(error) {
                return handleConnectionError(error, for: request)
            }

            if errorIsDeviceIsOffline(error) {
                return handleDeviceIsOfflineError(error, for: request)
            }

            if let urlError = error as? URLError, urlError.code == .cancelled {
                return .failure(
                    .RequestExecutionFailed(
                        statusCode: nil,
                        redirects: nil,
                        reason: .cancellationError,
                        requestUrl: request.url(),
                        requestMethod: request.method()
                    )
                )
            }

            if let urlError = error as? URLError, urlError.code == .timedOut {
                // `.timedOut` conflates connect- and read-timeouts, matching reqwest's
                // `is_timeout()` and Kotlin's `SocketTimeoutException`, so classifying it as
                // `httpTimeoutError` brings Apple platforms to parity with the other executors.
                // Caveat: with `URLSessionConfiguration.waitsForConnectivity == true` (the default is
                // `false`), an offline device surfaces here as `.timedOut` rather than
                // `.notConnectedToInternet`, so it classifies as `httpTimeoutError` rather than
                // `deviceIsOfflineError`.
                return .failure(
                    .RequestExecutionFailed(
                        statusCode: nil,
                        redirects: nil,
                        reason: .httpTimeoutError,
                        requestUrl: request.url(),
                        requestMethod: request.method()
                    )
                )
            }

            return .failure(
                .RequestExecutionFailed(
                    statusCode: nil,
                    redirects: nil,
                    reason: .genericError(errorMessage: error.localizedDescription),
                    requestUrl: request.url(),
                    requestMethod: request.method()
                )
            )
        }
    }

    private func fetch(request: URLRequest) async throws -> (Data, URLResponse) {
        #if os(Linux)
        return try await session.data(for: request)
        #else
        return try await session.data(for: request, delegate: executorDelegate)
        #endif
    }

    #if PROGRESS_REPORTING_ENABLED
    public func progresses(for context: RequestContext) -> AnyPublisher<Progress, Never> {
        NotificationCenter.default.publisher(for: RequestExecutorDelegate.didCreateTaskNotification)
            .compactMap { $0.object as? URLSessionTask }
            .filter {
                guard let requestId = $0.originalRequest?.requestId else { return false }

                return context.requestIds().contains(requestId)
            }
            .map { $0.progress }
            .eraseToAnyPublisher()
    }
    #endif

    private func cancelRequest(withId requestId: String) async {
        #if canImport(Combine)
        var task = (await self.session.allTasks)
            .first {
                $0.originalRequest?.requestId == requestId
            }

        if task == nil {
            task = await NotificationCenter.default
                .publisher(for: RequestExecutorDelegate.didCreateTaskNotification)
                .compactMap { $0.object as? URLSessionTask }
                .first { $0.originalRequest?.requestId == requestId }
                .timeout(.seconds(1), scheduler: DispatchQueue.global())
                .values
                .first { _ in true }
        }

        task?.cancel()
        #endif
    }

    private func handleHttpsError(
        _ error: Error,
        for request: NetworkRequestContent
    ) -> Result<WpNetworkResponse, RequestExecutionError> {

        guard let siteCertificate = leafCertificate(from: error) else {
            return .failure(
                .RequestExecutionFailed(
                    statusCode: nil,
                    redirects: executorDelegate.redirects(for: request.requestId()),
                    reason: .invalidSslError(reason: InvalidSslErrorReason.genericSslError),
                    requestUrl: request.url(),
                    requestMethod: request.method()
                )
            )
        }

        return .failure(
            .RequestExecutionFailed(
                statusCode: nil,
                redirects: executorDelegate.redirects(for: request.requestId()),
                reason: RequestExecutionErrorReason.invalidSslError(
                    reason: .certificateNotValidForName(
                        hostname: URL(string: request.url())?.host ?? "unknown host",
                        presentedHostnames: siteCertificate.presentedHostnames()
                    )
                ),
                requestUrl: request.url(),
                requestMethod: request.method()
            )
        )
    }

    func handleNonExistentSiteError(
        _ error: Error,
        for request: NetworkRequestContent
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        .failure(
            .RequestExecutionFailed(
                statusCode: nil,
                redirects: executorDelegate.redirects(for: request.requestId()),
                reason: .nonExistentSiteError(
                    errorMessage: error.localizedDescription,
                    suggestedAction: (error as NSError).localizedRecoverySuggestion
                ),
                requestUrl: request.url(),
                requestMethod: request.method()
            )
        )
    }

    func handleConnectionError(
        _ error: Error,
        for request: NetworkRequestContent
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        .failure(
            .RequestExecutionFailed(
                statusCode: nil,
                redirects: executorDelegate.redirects(for: request.requestId()),
                reason: .connectionError(reason: error.localizedDescription),
                requestUrl: request.url(),
                requestMethod: request.method()
            )
        )
    }

    public func sleep(millis: UInt64) async {
        // `try?`: `Task.sleep` only throws on cancellation, which this non-throwing `sleep`
        // leaves to the request machinery to handle.
        try? await Task.sleep(for: .milliseconds(millis))
    }

    private func errorIsHttpsError(_ error: Error) -> Bool {
        guard let urlError = error as? URLError else {
            return false
        }

        return [
            .secureConnectionFailed,
            .serverCertificateUntrusted,
            .serverCertificateHasBadDate,
            .serverCertificateNotYetValid,
            .serverCertificateHasUnknownRoot
        ]
        .contains(urlError.code)
    }

    private func errorIsNonExistentSiteError(_ error: Error) -> Bool {
        // A refused connection (`.cannotConnectToHost` — the host resolves, but
        // nothing is listening) is deliberately *not* here: it is a failed
        // connection handled by `errorIsConnectionError`, matching the Kotlin and
        // reqwest executors. This keeps `NonExistentSiteError` — and the
        // `isSiteUnreachable` predicate built on it — a portable "the host does
        // not resolve" signal across platforms. See #1495.
        //
        // `.badURL` is grouped here deliberately. A malformed URL has no dedicated
        // classification at the executor layer: `RequestExecutionError` can't
        // produce `WpApiError.SiteUrlParsingError` (that's a parse-time error, one
        // layer up) and `RequestExecutionErrorReason` has no invalid-URL case, so
        // `NonExistentSiteError` is the nearest fit. In practice we could not
        // construct a URL that reaches this branch: request URLs are normalized by
        // the Rust `url` crate before they arrive, and modern Foundation repairs
        // the leftovers (e.g. an invalid `%zz` becomes `%25zz`) rather than raising
        // `.badURL`. It's kept for completeness.
        [
            .badURL,
            .cannotFindHost,
            .dnsLookupFailed
        ]
        .contains((error as? URLError)?.code)
    }

    private func errorIsConnectionError(_ error: Error) -> Bool {
        // A failed connection: the host resolves, but nothing accepts the
        // connection (server down, wrong port, not listening, or no route). It
        // maps to `ConnectionError` — the same classification the Kotlin
        // (`ConnectException` / `NoRouteToHostException`) and reqwest (io-error)
        // executors use. `isSiteUnreachable` covers it alongside a DNS failure.
        [
            .cannotConnectToHost
        ]
        .contains((error as? URLError)?.code)
    }

    private func errorIsDeviceIsOffline(_ error: Error) -> Bool {
        // The device can't use the network right now; the site itself is not
        // implicated. Two codes mean there's no connection at all
        // (`.notConnectedToInternet`, and `.networkConnectionLost` for a connection
        // severed mid-request); three more mean the OS is refusing network use even
        // though the hardware is present — cellular data disallowed for the app or by
        // carrier policy (`.dataNotAllowed`, the common Wi-Fi-off case), roaming
        // turned off while abroad (`.internationalRoamingOff`), or a voice call
        // holding the radio on an older single-radio device (`.callIsActive`). This
        // matches what Kotlin callers get through the `NetworkAvailabilityProvider`
        // gate. See #1501.
        [
            .networkConnectionLost,
            .notConnectedToInternet,
            .dataNotAllowed,
            .internationalRoamingOff,
            .callIsActive
        ]
        .contains((error as? URLError)?.code)
    }

    private func handleDeviceIsOfflineError(
        _ error: Error,
        for request: NetworkRequestContent
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        .failure(
            .RequestExecutionFailed(
                statusCode: nil,
                redirects: nil,
                reason: .deviceIsOfflineError(
                    errorMessage: error.localizedDescription
                ),
                requestUrl: request.url(),
                requestMethod: request.method()
            )
        )
    }

    /// Parse the site (leaf) certificate out of a failed TLS handshake.
    ///
    /// The peer certificate chain is leaf-first, so the site's certificate is
    /// element 0 of the *raw* chain. We parse that element directly rather than
    /// parsing the whole chain and taking element 0 of whatever survived: a leaf
    /// we can't parse must degrade to `genericSslError`, never silently promote an
    /// intermediate CA's certificate into the site's position and report the CA's
    /// name (e.g. `R10`) as the presented hostname.
    private func leafCertificate(from error: Error) -> SslCertificateInfo? {
        #if os(Linux) // Linux doesn't support `SecCertificate`
        return nil
        #else
        // The certificate chain the server presented during the failed TLS handshake.
        guard
            let trust = (error as? URLError)?.failureURLPeerTrust,
            let certChainArray = SecTrustCopyCertificateChain(trust) as? [SecCertificate],
            let leaf = certChainArray.first
        else {
            return nil
        }

        return parseCertificate(data: SecCertificateCopyData(leaf) as Data)
        #endif
    }
}

private final class RequestExecutorDelegate:
    NSObject, URLSessionTaskDelegate, URLSessionDataDelegate, @unchecked Sendable
{

    static let didCreateTaskNotification = Notification.Name("RequestExecutorDelegate.didCreateTaskNotification")

    private let lock = NSLock()
    private var redirects: [String: [WpRedirect]] = [:]
    let delegate: URLSessionTaskDelegate?
    // When a site's domain is not included in its SSL certificate's common name and alternative names,
    // URLSession rejects the HTTPs connection. Here we allow the consumers to add additional
    // alternative names.
    // The key is SSL certificate common name.
    private var additionalAlternativeNames: [String: Set<String>] = [:]

    init(delegate: URLSessionTaskDelegate?, redirects: [String: [WpRedirect]] = [:]) {
        self.delegate = delegate
        self.redirects = redirects
    }

    func redirects(for taskID: String) -> [WpRedirect]? {
        lock.withLock {
            redirects[taskID]
        }
    }

    func allowSSL(altNames: [String], forCommonName name: String) {
        lock.withLock {
            additionalAlternativeNames[name, default: []].formUnion(altNames)
        }
    }

    private func alternateNames(forCertificate cert: SslCertificateInfo) -> Set<String> {
        // The allowlist is keyed on the certificate's Common Name, so a SAN-only
        // certificate that omits its CN can't be matched here — there's no key to
        // look it up under. Fall through to default handling in that case.
        guard let commonName = cert.commonName() else { return [] }
        return lock.withLock {
            additionalAlternativeNames[commonName] ?? []
        }
    }

    #if !os(Linux)
    func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        didReceive challenge: URLAuthenticationChallenge
    ) async -> (URLSession.AuthChallengeDisposition, URLCredential?) {
        if challenge.protectionSpace.authenticationMethod == NSURLAuthenticationMethodServerTrust,
            let trust = challenge.protectionSpace.serverTrust,
            let certificateChain = SecTrustCopyCertificateChain(trust) as? [SecCertificate],
            let cert = certificateChain.first,
            let cert = parseCertificate(data: SecCertificateCopyData(cert) as Data),
            alternateNames(forCertificate: cert).contains(challenge.protectionSpace.host)
        {
            return (.useCredential, URLCredential(trust: trust))
        }

        return (.performDefaultHandling, nil)
    }
    #endif

    #if !os(Linux)
    func urlSession(_ session: URLSession, didCreateTask task: URLSessionTask) {
        NotificationCenter.default.post(name: RequestExecutorDelegate.didCreateTaskNotification, object: task)

        delegate?.urlSession?(session, didCreateTask: task)
    }
    #endif

    func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        willPerformHTTPRedirection response: HTTPURLResponse,
        newRequest request: URLRequest
    ) async -> URLRequest? {

        guard
            let requestID = task.originalRequest?.requestId,
            let source = task.originalRequest?.url,
            let destination = request.url
        else {
            return request
        }

        lock.withLock {
            if redirects[requestID] == nil {
                redirects[requestID] = [WpRedirect]()
            }

            redirects[requestID]?
                .append(
                    WpRedirect(
                        source: source.absoluteString,
                        destination: destination.absoluteString
                    )
                )
        }

        return request
    }

    func urlSession(_ session: URLSession, taskIsWaitingForConnectivity task: URLSessionTask) {
        #if !os(Linux)
        delegate?.urlSession?(session, taskIsWaitingForConnectivity: task)
        #endif
    }

    func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        didSendBodyData bytesSent: Int64,
        totalBytesSent: Int64,
        totalBytesExpectedToSend: Int64
    ) {
        #if os(Linux)
        delegate?
            .urlSession(
                session,
                task: task,
                didSendBodyData: bytesSent,
                totalBytesSent: totalBytesSent,
                totalBytesExpectedToSend: totalBytesExpectedToSend
            )
        #else
        delegate?.urlSession?(
            session,
            task: task,
            didSendBodyData: bytesSent,
            totalBytesSent: totalBytesSent,
            totalBytesExpectedToSend: totalBytesExpectedToSend
        )
        #endif
    }

    func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        didReceiveInformationalResponse response: HTTPURLResponse
    ) {
        #if os(macOS)
        if #available(macOS 14.0, *) {
            delegate?.urlSession?(session, task: task, didReceiveInformationalResponse: response)
        }
        #elseif os(iOS)
        if #available(iOS 17.0, *) {
            delegate?.urlSession?(session, task: task, didReceiveInformationalResponse: response)
        }
        #endif
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didFinishCollecting metrics: URLSessionTaskMetrics) {
        #if os(Linux)
        delegate?.urlSession(session, task: task, didFinishCollecting: metrics)
        #else
        delegate?.urlSession?(session, task: task, didFinishCollecting: metrics)
        #endif
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: (any Error)?) {
        #if os(Linux)
        delegate?.urlSession(session, task: task, didCompleteWithError: error)
        #else
        delegate?.urlSession?(session, task: task, didCompleteWithError: error)
        #endif
    }

    func urlSession(
        _ session: URLSession,
        dataTask: URLSessionDataTask,
        didReceive response: URLResponse,
        completionHandler: @escaping @Sendable (URLSession.ResponseDisposition) -> Void
    ) {
        #if os(Linux)
        (delegate as? URLSessionDataDelegate)?
            .urlSession(
                session,
                dataTask: dataTask,
                didReceive: response,
                completionHandler: { _ in }
            )
        #else
        (delegate as? URLSessionDataDelegate)?.urlSession?(
            session,
            dataTask: dataTask,
            didReceive: response,
            completionHandler: { _ in }
        )
        #endif

        completionHandler(.allow)
    }

    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask, didReceive data: Data) {
        #if os(Linux)
        (delegate as? URLSessionDataDelegate)?.urlSession(session, dataTask: dataTask, didReceive: data)
        #else
        (delegate as? URLSessionDataDelegate)?.urlSession?(session, dataTask: dataTask, didReceive: data)
        #endif
    }
}

private let requestIdHeaderName = "X-REQUEST-ID"

extension URLRequest {
    var requestId: String? {
        allHTTPHeaderFields?[requestIdHeaderName]
    }
}

protocol NetworkRequestContent {
    func requestId() -> String
    func method() -> RequestMethod
    func url() -> WpEndpointUrl
    func headerMap() -> WpNetworkHeaderMap
    func encodeBody(into request: inout URLRequest) throws

    func perform(
        in session: URLSession,
        withAdditionalHeaders: [String: String],
        delegate: URLSessionTaskDelegate?
    ) async throws -> (Data, URLResponse)
}

extension NetworkRequestContent {
    func buildURLRequest(additionalHeaders: [String: String]) throws -> URLRequest {
        let url = URL(string: self.url())!
        var request = URLRequest(url: url)
        request.httpMethod = self.method().rawValue
        request.allHTTPHeaderFields = self.headerMap().toFlatMap()
        request.allHTTPHeaderFields?[requestIdHeaderName] = self.requestId()
        for (name, value) in additionalHeaders {
            request.addValue(value, forHTTPHeaderField: name)
        }
        try self.encodeBody(into: &request)
        return request
    }
}

extension WpNetworkRequest: NetworkRequestContent {

    func encodeBody(into request: inout URLRequest) throws {
        if let body = self.body()?.contents() {
            request.httpBody = body
        }
    }

    func perform(
        in session: URLSession,
        withAdditionalHeaders headers: [String: String],
        delegate: URLSessionTaskDelegate?
    ) async throws -> (Data, URLResponse) {
        let request = try buildURLRequest(additionalHeaders: headers)

        let cancellation = TaskCancellation()
        return try await withTaskCancellationHandler {
            let result: Result<(Data, URLResponse), Error> = await withCheckedContinuation { continuation in
                let task = session.dataTask(with: request, completionHandler: completionHandler(continuation))
                cancellation.task = task

                // See https://github.com/Automattic/wordpress-rs/pull/1046
                #if !os(Linux)
                task.delegate = delegate
                #endif

                task.resume()

                #if !os(Linux)
                delegate?.urlSession?(session, didCreateTask: task)
                #endif
            }

            if let task = cancellation.task {
                notifyTaskResult(delegate: delegate, session: session, task: task, result: result)
            }

            return try result.get()
        } onCancel: {
            cancellation.cancel()
        }
    }
}

extension MultipartFormError {
    /// The executor error this serialization failure maps to, or `nil` to fall through
    /// to `.genericError`. A file-backed field with a read failure (non-nil `filePath`)
    /// becomes `.MediaFileUnreadable`; an in-memory field (nil path) or `.impossible`
    /// has no file to name and stays generic.
    var asRequestExecutionError: RequestExecutionError? {
        switch self {
        case let .inaccessibleFile(_, filePath?):
            return .MediaFileUnreadable(filePath: filePath)
        case .inaccessibleFile(_, nil), .impossible:
            return nil
        }
    }
}

extension WpMultipartFormRequest: NetworkRequestContent {

    func encodeBody(into request: inout URLRequest) throws {
        // Do nothing.
    }

    func perform(
        in session: URLSession,
        withAdditionalHeaders headers: [String: String],
        delegate: URLSessionTaskDelegate?
    ) async throws -> (Data, URLResponse) {
        var request = try buildURLRequest(additionalHeaders: headers)

        var form = [MultipartFormField]()
        for field in self.form() {
            switch field {
            case .text(let name, let value):
                form.append(MultipartFormField(text: value, name: name))
            case .file(let name, let file):
                var mimeType = file.mimeType

                #if canImport(UniformTypeIdentifiers)
                if mimeType == nil {
                    mimeType =
                        UTType(
                            filenameExtension: URL(fileURLWithPath: file.filePath).pathExtension
                        )?
                        .preferredMIMEType
                }
                #endif

                do {
                    try form.append(
                        .init(
                            fileAtPath: file.filePath,
                            name: name,
                            filename: file.fileName,
                            mimeType: mimeType
                        )
                    )
                } catch {
                    throw RequestExecutionError.MediaFileNotFound(filePath: file.filePath)
                }
            }
        }

        let boundery = String(format: "wordpressrs.%08x", Int.random(in: Int.min..<Int.max))
        request.setValue("multipart/form-data; boundary=\(boundery)", forHTTPHeaderField: "Content-Type")

        let body: MultipartFormContent
        do {
            body = try form.multipartFormDataStream(boundary: boundery, forceWriteToFile: false)
        } catch let error as MultipartFormError {
            // Map a mid-read failure to `.MediaFileUnreadable` (see `asRequestExecutionError`);
            // anything with no file to name re-throws and falls through to `.genericError`.
            throw error.asRequestExecutionError ?? error
        }

        // `upload(...)` owns the temp file's lifetime and removes it once the transfer finishes. Arm
        // cleanup here too, where serialization acquires the file, so a future edit that throws
        // between this line and the `upload(...)` call below can't silently reintroduce the #1540
        // leak. Idempotent with upload's own `defer`: a second `removeItem` on an already-deleted
        // file is a harmless no-op.
        defer { body.removeTemporaryFileIfNeeded() }

        let mediaFilePath = self.form().lazy
            .compactMap { field -> String? in
                if case let .file(_, file) = field { file.filePath } else { nil }
            }
            .first
        let bodyLength = try body.length(reportingFailureAgainst: mediaFilePath)

        return try await upload(
            body: body,
            bodyLength: bodyLength,
            with: request,
            session: session,
            delegate: delegate
        )
    }
}

/// Uploads a serialized multipart `body` — either in memory or a temporary file on disk — over
/// `URLSession`, awaiting completion.
///
/// A `.onDisk` body must be cleaned up once the transfer finishes. `URLSession`'s
/// `uploadTask(fromFile:)` reads the file asynchronously for the duration of the transfer and never
/// deletes it, so the file has to outlive serialization (it can't be removed in
/// `multipartFormDataStream`) and can only be removed here, after the upload completes on any path —
/// success, failure, or cancellation. Without this, every large upload leaks a temp file (#1540).
///
/// On Apple platforms the task is delegate-based rather than completion-handler-based, so that a
/// server that answers before the body is fully sent (for example, a proxy rejecting a large upload
/// with a `413`) is returned to the caller instead of timing out. See `UploadTaskDelegate`.
///
/// `bodyLength` is the size of `body` in bytes. If it's `nil`, it's read here, and a failure to read
/// it is thrown.
///
/// This is a free function rather than a method because it needs no state from the request, which
/// also lets tests drive it directly against a stubbed `URLSession`.
func upload(
    body: MultipartFormContent,
    bodyLength: Int64? = nil,
    with request: URLRequest,
    session: URLSession,
    delegate: URLSessionTaskDelegate?
) async throws -> (Data, URLResponse) {
    defer { body.removeTemporaryFileIfNeeded() }

    #if !os(Linux)
    let bodyLength = try bodyLength ?? body.length()
    #endif

    let cancellation = TaskCancellation()
    return try await withTaskCancellationHandler {
        let result: Result<(Data, URLResponse), Error> = await withCheckedContinuation { continuation in
            #if os(Linux)
            // FoundationNetworking's per-task delegate support differs, and the early-response hang
            // `UploadTaskDelegate` works around is an Apple `URLSession` behaviour.
            let completion = completionHandler(continuation)
            let task =
                switch body {
                case let .inMemory(data):
                    session.uploadTask(with: request, from: data, completionHandler: completion)
                case let .onDisk(file):
                    session.uploadTask(with: request, fromFile: file, completionHandler: completion)
                }
            cancellation.task = task
            task.resume()
            #else
            let task =
                switch body {
                case let .inMemory(data):
                    session.uploadTask(with: request, from: data)
                case let .onDisk(file):
                    session.uploadTask(with: request, fromFile: file)
                }
            task.delegate = UploadTaskDelegate(
                forwardingTo: delegate,
                bodyLength: bodyLength,
                continuation: continuation
            )
            // After the delegate is set: this cancels the task at once if the caller already has.
            cancellation.task = task
            task.resume()

            delegate?.urlSession?(session, didCreateTask: task)
            #endif
        }

        #if os(Linux)
        if let task = cancellation.task {
            notifyTaskResult(delegate: delegate, session: session, task: task, result: result)
        }
        #endif

        return try result.get()
    } onCancel: {
        cancellation.cancel()
    }
}

extension MultipartFormContent {
    /// The size of the body in bytes. Throws if the temp file's size can't be read.
    func length() throws -> Int64 {
        switch self {
        case let .inMemory(data):
            return Int64(data.count)
        case let .onDisk(url):
            guard let size = try FileManager.default.attributesOfItem(atPath: url.path)[.size] as? NSNumber else {
                throw CocoaError(.fileReadUnknown, userInfo: [NSFilePathErrorKey: url.path])
            }
            return size.int64Value
        }
    }

    /// `length()`, with a failure reported as `.MediaFileUnreadable` for `mediaFilePath`, the media
    /// file the body was serialized from.
    ///
    /// The temp file was written a moment ago, so this failing is unlikely. If it does, Foundation's
    /// own error names the temp file, which means nothing to the user. With no `mediaFilePath`, that
    /// error is thrown as it is.
    func length(reportingFailureAgainst mediaFilePath: String?) throws -> Int64 {
        do {
            return try length()
        } catch {
            guard let mediaFilePath else { throw error }
            throw RequestExecutionError.MediaFileUnreadable(filePath: mediaFilePath)
        }
    }
}

#if !os(Linux)
/// The per-task delegate for `upload(body:with:session:delegate:)`. It collects the response, and
/// returns it as soon as it's complete if the server answered before the request body was fully sent.
///
/// A server or proxy can reject an upload before reading all of it — Cloudflare answers an oversized
/// upload with a `413` after about 1–2 MB. Over HTTP/3 (and over HTTP/1.1 when the server keeps the
/// connection open), `URLSession` receives that response but never completes the task: it keeps
/// waiting to send the rest of the body, then fails with `URLError.timedOut` once the request
/// timeout elapses. A completion-handler task only ever sees that timeout. A delegate-based task is
/// told about the response, so this delegate keeps it, cancels the task, and returns the response as
/// a success. The Rust layer then classifies its status code as it would any other response.
///
/// Only a rejection — a status outside `2xx` — is returned early. A server can't accept an upload it
/// hasn't read, so an early-looking `2xx` is one whose sent-byte counts lagged the socket, and the
/// task completes it normally.
///
/// Every callback is forwarded to `forwarding` (the executor's `RequestExecutorDelegate`), which
/// handles TLS trust, records redirects and notifies the app's delegate. Because `URLSession`
/// delivers `didReceive response`, `didReceive data` and `didCompleteWithError` itself, the manual
/// `notifyTaskResult(...)` replay isn't used on this path, so the app's delegate sees each event once.
///
/// When an early response is returned, `URLSession` later reports the cancellation (or whatever
/// error ended the stalled upload). The app's delegate is told the task completed without an error
/// instead, so it records the same outcome the caller received: the server's response.
///
/// Two kinds of early response still time out, because `URLSession` never delivers them to this
/// delegate (seen over HTTP/1.1):
/// - one with neither a `Content-Length` nor a `Content-Type`, which it holds back to sniff its
///   type from more of the body;
/// - one with an empty body, after which it reports the whole request body as sent.
/// Proxies' error pages have a body and a `Content-Type`.
///
/// Over HTTP/1.1, cancelling the stalled upload can also break the *next* request on the session if
/// the server kept the connection alive: `URLSession` hands that request the cancelled upload's
/// connection, and it fails with `URLError.networkConnectionLost`. Servers normally send
/// `Connection: close` with a rejection (nginx does), which avoids it; setting it on the request
/// doesn't. HTTP/2 and HTTP/3 multiplex streams, so this doesn't apply to them.
///
/// Returning early removes an `.onDisk` body's temp file while the cancelled task may still have it
/// open. That's harmless: unlinking an open file leaves it readable through the open descriptor.
private final class UploadTaskDelegate: NSObject, URLSessionDataDelegate, @unchecked Sendable {
    /// How long to wait for more of an early response's body when it has no `Content-Length`. Error
    /// pages from proxies are small and arrive with their headers, so this only bounds a wait that
    /// shouldn't happen; it's well below the 60 s default request timeout it replaces.
    static let earlyResponseQuietPeriod: DispatchTimeInterval = .seconds(1)

    private let forwarding: URLSessionTaskDelegate?
    private let bodyLength: Int64

    private let lock = NSLock()
    private var continuation: CheckedContinuation<Result<(Data, URLResponse), Error>, Never>?
    private var outcome: Result<(Data, URLResponse), Error>?
    private var response: URLResponse?
    private var data = Data()
    private var totalBytesSent: Int64 = 0
    private var isEarlyResponse = false
    private var quietPeriod: DispatchWorkItem?

    init(
        forwardingTo forwarding: URLSessionTaskDelegate?,
        bodyLength: Int64,
        continuation: CheckedContinuation<Result<(Data, URLResponse), Error>, Never>
    ) {
        self.forwarding = forwarding
        self.bodyLength = bodyLength
        self.continuation = continuation
    }

    /// Resumes the caller with `result`, unless it has already been resumed.
    private func finish(_ result: Result<(Data, URLResponse), Error>) {
        let continuation: CheckedContinuation<Result<(Data, URLResponse), Error>, Never>? = lock.withLock {
            guard outcome == nil else { return nil }
            outcome = result
            quietPeriod?.cancel()
            quietPeriod = nil
            defer { self.continuation = nil }
            return self.continuation
        }
        continuation?.resume(returning: result)
    }

    /// Whether the request body has been fully sent. `countOfBytesExpectedToSend` can still be `0`
    /// when the response arrives, so compare against the body's own length.
    ///
    /// The sent-byte counts can lag the socket, so a server that did read the whole body can still
    /// look like it answered early. That's why a `2xx` is never treated as early: returning it
    /// early would trust the quiet period to judge when its body is complete, and a success body
    /// that pauses for longer than that would be cut short.
    private func isBodyFullySent(_ task: URLSessionTask) -> Bool {
        let sent = max(task.countOfBytesSent, lock.withLock { totalBytesSent })
        return sent >= bodyLength
    }

    /// If the server answered before the body was fully sent, returns its response once its body is
    /// complete: when `Content-Length` bytes have arrived, or, without a `Content-Length`, after a
    /// quiet period with no new data. A body that stops short of its `Content-Length` is left to
    /// `didCompleteWithError`, which returns what arrived once the task fails or times out.
    private func returnEarlyResponseIfComplete(_ task: URLSessionTask) {
        enum Completeness { case complete, unknown, incomplete }
        let completeness: Completeness? = lock.withLock {
            guard outcome == nil, isEarlyResponse, let response else { return nil }
            quietPeriod?.cancel()
            quietPeriod = nil
            let expectedLength = response.expectedContentLength
            guard expectedLength >= 0 else { return .unknown }
            return Int64(data.count) >= expectedLength ? .complete : .incomplete
        }

        let work = DispatchWorkItem { [weak self, weak task] in
            self?.returnEarlyResponse(cancelling: task)
        }
        switch completeness {
        case .complete:
            work.perform()
        case .unknown:
            lock.withLock { quietPeriod = work }
            DispatchQueue.global().asyncAfter(deadline: .now() + Self.earlyResponseQuietPeriod, execute: work)
        case .incomplete, nil:
            break
        }
    }

    private func returnEarlyResponse(cancelling task: URLSessionTask?) {
        guard let (data, response) = lock.withLock({ self.response.map { (self.data, $0) } }) else { return }
        // Resume the caller before cancelling, so the cancellation `URLSession` reports for it finds
        // the outcome already decided.
        finish(.success((data, response)))
        task?.cancel()
    }

    // The challenge and redirect callbacks use their completion-handler forms: forwarding the
    // `async` forms through an optional protocol requirement crashes the Swift 6.4 compiler in SILGen.
    func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        didReceive challenge: URLAuthenticationChallenge,
        completionHandler: @escaping @Sendable (URLSession.AuthChallengeDisposition, URLCredential?) -> Void
    ) {
        guard let forward = forwarding?.urlSession(_:task:didReceive:completionHandler:) else {
            completionHandler(.performDefaultHandling, nil)
            return
        }
        forward(session, task, challenge, completionHandler)
    }

    func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        willPerformHTTPRedirection response: HTTPURLResponse,
        newRequest request: URLRequest,
        completionHandler: @escaping @Sendable (URLRequest?) -> Void
    ) {
        guard let forward = forwarding?.urlSession(_:task:willPerformHTTPRedirection:newRequest:completionHandler:)
        else {
            completionHandler(request)
            return
        }
        forward(session, task, response, request, completionHandler)
    }

    func urlSession(_ session: URLSession, taskIsWaitingForConnectivity task: URLSessionTask) {
        forwarding?.urlSession?(session, taskIsWaitingForConnectivity: task)
    }

    func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        didSendBodyData bytesSent: Int64,
        totalBytesSent: Int64,
        totalBytesExpectedToSend: Int64
    ) {
        lock.withLock { self.totalBytesSent = max(self.totalBytesSent, totalBytesSent) }
        forwarding?.urlSession?(
            session,
            task: task,
            didSendBodyData: bytesSent,
            totalBytesSent: totalBytesSent,
            totalBytesExpectedToSend: totalBytesExpectedToSend
        )
    }

    func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        didReceiveInformationalResponse response: HTTPURLResponse
    ) {
        #if os(macOS)
        if #available(macOS 14.0, *) {
            forwarding?.urlSession?(session, task: task, didReceiveInformationalResponse: response)
        }
        #elseif os(iOS)
        if #available(iOS 17.0, *) {
            forwarding?.urlSession?(session, task: task, didReceiveInformationalResponse: response)
        }
        #endif
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didFinishCollecting metrics: URLSessionTaskMetrics) {
        forwarding?.urlSession?(session, task: task, didFinishCollecting: metrics)
    }

    func urlSession(
        _ session: URLSession,
        dataTask: URLSessionDataTask,
        didReceive response: URLResponse,
        completionHandler: @escaping @Sendable (URLSession.ResponseDisposition) -> Void
    ) {
        let isRejection = (response as? HTTPURLResponse).map { !(200..<300).contains($0.statusCode) } ?? false
        let isEarlyResponse = isRejection && !isBodyFullySent(dataTask)
        lock.withLock {
            self.response = response
            self.isEarlyResponse = isEarlyResponse
        }

        (forwarding as? URLSessionDataDelegate)?.urlSession?(
            session,
            dataTask: dataTask,
            didReceive: response,
            completionHandler: { _ in }
        )
        completionHandler(.allow)

        returnEarlyResponseIfComplete(dataTask)
    }

    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask, didReceive data: Data) {
        lock.withLock { self.data.append(data) }
        (forwarding as? URLSessionDataDelegate)?.urlSession?(session, dataTask: dataTask, didReceive: data)
        returnEarlyResponseIfComplete(dataTask)
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: (any Error)?) {
        let (response, data, isEarlyResponse) = lock.withLock { (self.response, self.data, self.isEarlyResponse) }

        if let error {
            // An early response is preferred over the error that later ended the stalled upload
            // (`-1001`, `-1005`, …) — unless the caller cancelled the request.
            let isCancellation = (error as? URLError)?.code == .cancelled
            if isEarlyResponse, let response, !isCancellation {
                finish(.success((data, response)))
            } else {
                finish(.failure(error))
            }
        } else if let response = task.response ?? response {
            finish(.success((data, response)))
        } else {
            finish(.failure(URLError(.badServerResponse)))
        }

        // Report the outcome the caller received, which may have been decided before this callback
        // (an early response), rather than the raw error.
        let reportedError: Error? =
            switch lock.withLock({ outcome }) {
            case .failure(let error): error
            case .success, .none: nil
            }
        forwarding?.urlSession?(session, task: task, didCompleteWithError: reportedError)
    }
}
#endif

private class TaskCancellation: @unchecked Sendable {
    private let lock = NSLock()
    private var _task: URLSessionTask?
    private var isCancelled = false

    /// The task to cancel. A task set after `cancel()` — when the Swift task was already cancelled
    /// before the request started — is cancelled straight away, rather than left to run until it
    /// times out.
    var task: URLSessionTask? {
        get {
            lock.withLock { _task }
        }
        set {
            lock.withLock {
                _task = newValue
                if isCancelled { newValue?.cancel() }
            }
        }
    }

    func cancel() {
        lock.withLock {
            isCancelled = true
            _task?.cancel()
        }
    }
}

private func completionHandler(
    _ continuation: CheckedContinuation<Result<(Data, URLResponse), any Error>, Never>
) -> @Sendable (Data?, URLResponse?, Error?) -> Void {
    { (data, response, error) in
        if let error {
            continuation.resume(returning: .failure(error))
        } else {
            // It's okay to force-unwrap here.
            // swiftlint:disable:next line_length
            // https://github.com/swiftlang/swift-corelibs-foundation/blob/swift-6.2.1-RELEASE/Sources/FoundationNetworking/URLSession/URLSession.swift#L743
            continuation.resume(returning: .success((data!, response!)))
        }
    }
}

private func notifyTaskResult(
    delegate: URLSessionTaskDelegate?,
    session: URLSession,
    task: URLSessionTask,
    result: Result<(Data, URLResponse), any Error>
) {
    if let task = task as? URLSessionDataTask, let delegate = delegate as? URLSessionDataDelegate {
        if case let .success((data, response)) = result {
            #if os(Linux)
            delegate.urlSession(session, dataTask: task, didReceive: response, completionHandler: { _ in })
            delegate.urlSession(session, dataTask: task, didReceive: data)
            #else
            delegate.urlSession?(session, dataTask: task, didReceive: response, completionHandler: { _ in })
            delegate.urlSession?(session, dataTask: task, didReceive: data)
            #endif
        }
    }

    let error: Error? =
        if case let .failure(error) = result {
            error
        } else {
            nil
        }
    #if os(Linux)
    delegate?.urlSession(session, task: task, didCompleteWithError: error)
    #else
    delegate?.urlSession?(session, task: task, didCompleteWithError: error)
    #endif
}
