import Foundation
import WordPressAPIInternal

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

#if canImport(Combine)
import Combine
#endif

public protocol SafeRequestExecutor: RequestExecutor, Sendable {
    func execute(
        _ request: WpNetworkRequest,
        cancellationToken: CancellationToken?
    ) async -> Result<WpNetworkResponse, RequestExecutionError>
    func uploadMedia(
        mediaUploadRequest: MediaUploadRequest,
        cancellationToken: CancellationToken?
    ) async -> Result<WpNetworkResponse, MediaUploadRequestExecutionError>

    #if PROGRESS_REPORTING_ENABLED
    /// Returns a publisher that emits zero or one `Progress` instance representing the overall progress of the task
    /// for the given `requestId`.
    func progress(forRequestWithId requestId: String) -> AnyPublisher<Progress, Never>
    #endif
}

extension SafeRequestExecutor {
    public func execute(
        request: WpNetworkRequest,
        cancellationToken: CancellationToken?
    ) async throws -> WpNetworkResponse {
        let result = await execute(request, cancellationToken: cancellationToken)
        return try result.get()
    }

    public func uploadMedia(
        mediaUploadRequest: MediaUploadRequest,
        cancellationToken: CancellationToken?
    ) async throws -> WpNetworkResponse {
        let result = await uploadMedia(mediaUploadRequest: mediaUploadRequest, cancellationToken: cancellationToken)
        return try result.get()
    }
}

public final class WpRequestExecutor: SafeRequestExecutor {
    private let session: URLSession
    private let executorDelegate: RequestExecutorDelegate

    private let additionalHttpHeadersForAllRequests: [String: String]

    private let cancellationHandlers = CancellationHandlers()

    public init(
        urlSession: URLSession,
        additionalHttpHeadersForAllRequests: [String: String] = [:],
        userAgent: String = defaultUserAgent(clientSpecificPostfix: UserAgent.postfix)
    ) {
        self.session = urlSession
        self.executorDelegate = RequestExecutorDelegate()

        var headers = additionalHttpHeadersForAllRequests
        if !headers.contains(where: { $0.key.caseInsensitiveCompare("User-Agent") == .orderedSame }) {
            headers["User-Agent"] = userAgent
        }
        self.additionalHttpHeadersForAllRequests = headers
    }

    public func execute(
        _ request: WpNetworkRequest,
        cancellationToken: CancellationToken?
    ) async -> Result<WpNetworkResponse, RequestExecutionError> {
        await perform(request, cancellationToken: cancellationToken)
    }

    public func uploadMedia(
        mediaUploadRequest: MediaUploadRequest,
        cancellationToken: CancellationToken?
    ) async -> Result<WpNetworkResponse, MediaUploadRequestExecutionError> {
        (await perform(mediaUploadRequest, cancellationToken: cancellationToken))
            .mapError { error in
                switch error {
                case let .RequestExecutionFailed(statusCode, redirects, reason):
                    MediaUploadRequestExecutionError.RequestExecutionFailed(
                        statusCode: statusCode,
                        redirects: redirects,
                        reason: reason
                    )
                }
            }
    }

    func perform(
        _ request: NetworkRequestContent,
        cancellationToken: CancellationToken?
    ) async -> Result<WpNetworkResponse, RequestExecutionError> {
        if let cancellationToken {
            let requestId = request.requestId()
            await cancellationHandlers.whenCancelling(cancellationToken) {
                Task { [weak self] in
                    await self?.cancelRequest(withId: requestId)
                }
            }
        }

        let result = await perform(request)

        if let cancellationToken {
            await cancellationHandlers.remove(cancellationToken)
        }

        return result
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
            if errorIsHttpsError(error) {
                return handleHttpsError(error, for: request)
            }

            if errorIsNonExistentSiteError(error) {
                return handleNonExistentSiteError(error, for: request)
            }

            if errorIsDeviceIsOffline(error) {
                return handleDeviceIsOfflineError(error)
            }

            if let urlError = error as? URLError, urlError.code == .cancelled {
                return .failure(.RequestExecutionFailed(
                    statusCode: nil,
                    redirects: nil,
                    reason: .cancellationError
                ))
            }

            return .failure(.RequestExecutionFailed(
                statusCode: nil,
                redirects: nil,
                reason: .genericError(errorMessage: error.localizedDescription)
            ))
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
    public func progress(forRequestWithId requestId: String) -> AnyPublisher<Progress, Never> {
        NotificationCenter.default.publisher(for: RequestExecutorDelegate.didCreateTaskNotification)
            .compactMap { $0.object as? URLSessionTask }
            .first { $0.originalRequest?.requestId == requestId }
            .map { $0.progress }
            .eraseToAnyPublisher()
    }
#endif

    private func cancelRequest(withId requestId: String) async {
#if canImport(Combine)
        var task = (await self.session.allTasks).first {
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

        guard
            var peerCertificateChain = getPeerCertificateChain(error),
            !peerCertificateChain.isEmpty
        else {
            return .failure(.RequestExecutionFailed(
                 statusCode: nil,
                 redirects: executorDelegate.redirects(for: request.requestId()),
                 reason: .invalidSslError(reason: InvalidSslErrorReason.genericSslError)
            ))
        }

        let siteCertificate = peerCertificateChain.remove(at: 0)

        return .failure(.RequestExecutionFailed(
             statusCode: nil,
             redirects: executorDelegate.redirects(for: request.requestId()),
             reason: RequestExecutionErrorReason.invalidSslError(
                reason: .certificateNotValidForName(
                    hostname: URL(string: request.url())?.host ?? "unknown host",
                    presentedHostnames: [siteCertificate.commonName()]
                )
             )
         ))
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
                )
            )
        )
    }

    public func sleep(millis: UInt64) async {
        // swiftlint:disable:next force_try
        try! await Task.sleep(nanoseconds: millis * 1000)
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
        ].contains(urlError.code)
    }

    private func errorIsNonExistentSiteError(_ error: Error) -> Bool {
        [
            .badURL,
            .cannotConnectToHost,
            .cannotFindHost,
            .dnsLookupFailed
        ].contains((error as? URLError)?.code)
    }

    private func errorIsDeviceIsOffline(_ error: Error) -> Bool {
        [
            .networkConnectionLost,
            .notConnectedToInternet
        ].contains((error as? URLError)?.code)
    }

    private func handleDeviceIsOfflineError(
        _ error: Error
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        .failure(
            .RequestExecutionFailed(
                statusCode: nil,
                redirects: nil,
                reason: .deviceIsOfflineError(
                    errorMessage: error.localizedDescription
                )
            )
        )
    }

    private func getPeerCertificateChain(_ error: Error) -> [SSLCertificateInfo]? {
        let nserror = error as NSError
        let info = nserror.userInfo

        #if os(Linux) // Linux doesn't support `SecCertificate`
        return []
        #else
        guard let certChainArray = info["NSErrorPeerCertificateChainKey"] as? [SecCertificate] else {
            return nil
        }

        return certChainArray.compactMap { parseCertificate(data: SecCertificateCopyData($0) as Data) }
        #endif
    }
}

private final class RequestExecutorDelegate: NSObject, URLSessionTaskDelegate, @unchecked Sendable {

    static let didCreateTaskNotification = Notification.Name("RequestExecutorDelegate.didCreateTaskNotification")

    private let lock = NSLock()
    private var redirects: [String: [WpRedirect]] = [:]

    init(redirects: [String: [WpRedirect]] = [:]) {
        self.redirects = redirects
    }

    func redirects(for taskID: String) -> [WpRedirect]? {
        lock.withLock {
            redirects[taskID]
        }
    }

    func urlSession(_ session: URLSession, didCreateTask task: URLSessionTask) {
        NotificationCenter.default.post(name: RequestExecutorDelegate.didCreateTaskNotification, object: task)
    }

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

            redirects[requestID]?.append(WpRedirect(
                source: source.absoluteString,
                destination: destination.absoluteString
            ))
        }

        return request
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
        #if os(Linux)
        return try await session.data(for: request)
        #else
        return try await session.data(for: request, delegate: delegate)
        #endif
    }
}

extension MediaUploadRequest: NetworkRequestContent {

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
        for (name, value) in mediaParams() {
            form.append(.init(text: value, name: name))
        }
        try form.append(.init(fileAtPath: filePath(), name: "file"))

        let boundery = String(format: "wordpressrs.%08x", Int.random(in: Int.min..<Int.max))
        request.setValue("multipart/form-data; boundary=\(boundery)", forHTTPHeaderField: "Content-Type")
        let body = try form.multipartFormDataStream(boundary: boundery, forceWriteToFile: false)

        #if os(Linux)
        switch body {
        case let .inMemory(data):
            return try await session.upload(for: request, from: data)
        case let .onDisk(file):
            return try await session.upload(for: request, fromFile: file)
        }
        #else
        switch body {
        case let .inMemory(data):
            return try await session.upload(for: request, from: data, delegate: delegate)
        case let .onDisk(file):
            return try await session.upload(for: request, fromFile: file, delegate: delegate)
        }
        #endif
    }

}

private actor CancellationHandlers {
    private var handlers: [String /* CancellationToken.uuid */: [RequestCancellationHandler]] = [:]

    func whenCancelling(_ token: CancellationToken, closure: @escaping @Sendable () -> Void) {
        let handler = RequestCancellationHandler(closure: closure)
        handlers[token.uuid(), default: []].append(handler)
        try? token.registerHandler(handler: handler)
    }

    func remove(_ token: CancellationToken) {
        handlers.removeValue(forKey: token.uuid())
    }
}

private final class RequestCancellationHandler: CancellationHandler, Hashable {
    let closure: @Sendable () -> Void

    init(closure: @escaping @Sendable () -> Void) {
        self.closure = closure
    }

    func cancelled() {
        self.closure()
    }

    func hash(into hasher: inout Hasher) {
        hasher.combine(ObjectIdentifier(self))
    }

    static func == (lhs: RequestCancellationHandler, rhs: RequestCancellationHandler) -> Bool {
        ObjectIdentifier(lhs) == ObjectIdentifier(rhs)
    }
}
