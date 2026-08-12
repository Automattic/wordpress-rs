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
        [
            .networkConnectionLost,
            .notConnectedToInternet
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
        let body = try form.multipartFormDataStream(boundary: boundery, forceWriteToFile: false)
        return try await upload(body: body, with: request, session: session, delegate: delegate)
    }

    private func upload(
        body: MultipartFormContent,
        with request: URLRequest,
        session: URLSession,
        delegate: URLSessionTaskDelegate?
    ) async throws -> (Data, URLResponse) {
        let cancellation = TaskCancellation()
        return try await withTaskCancellationHandler {
            let result: Result<(Data, URLResponse), Error> = await withCheckedContinuation { continuation in
                let completion = completionHandler(continuation)
                let task =
                    switch body {
                    case let .inMemory(data):
                        session.uploadTask(with: request, from: data, completionHandler: completion)
                    case let .onDisk(file):
                        session.uploadTask(with: request, fromFile: file, completionHandler: completion)
                    }
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

private class TaskCancellation: @unchecked Sendable {
    private let lock = NSLock()
    private var _task: URLSessionTask?

    var task: URLSessionTask? {
        get {
            lock.withLock { _task }
        }
        set {
            lock.withLock { _task = newValue }
        }
    }

    func cancel() {
        lock.withLock {
            _task?.cancel()
            _task = nil
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
