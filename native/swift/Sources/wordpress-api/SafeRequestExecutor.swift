import Foundation
import WordPressAPIInternal

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

public protocol SafeRequestExecutor: RequestExecutor, Sendable {
    func execute(_ request: WpNetworkRequest) async -> Result<WpNetworkResponse, RequestExecutionError>
    func uploadMedia(mediaUploadRequest: MediaUploadRequest) async -> Result<WpNetworkResponse, MediaUploadRequestExecutionError>
}

extension SafeRequestExecutor {
    public func execute(request: WpNetworkRequest) async throws -> WpNetworkResponse {
        let result = await execute(request)
        return try result.get()
    }

    public func uploadMedia(mediaUploadRequest: MediaUploadRequest) async throws -> WpNetworkResponse {
        let result = await uploadMedia(mediaUploadRequest: mediaUploadRequest)
        return try result.get()
    }
}

public final class WpRequestExecutor: SafeRequestExecutor {
    private let session: URLSession
    private let executorDelegate: RequestExecutorDelegate

    private let additionalHttpHeadersForAllRequests: [String: String]

    private let userAgent: String

    public init(
        urlSession: URLSession,
        additionalHttpHeadersForAllRequests: [String: String] = [:],
        userAgent: String = defaultUserAgent(clientSpecificPostfix: UserAgent.postfix)
    ) {
        self.session = urlSession
        self.executorDelegate = RequestExecutorDelegate()
        self.userAgent = userAgent
        self.additionalHttpHeadersForAllRequests = additionalHttpHeadersForAllRequests
    }

    public func execute(_ request: WpNetworkRequest) async -> Result<WpNetworkResponse, RequestExecutionError> {
        await perform(request)
    }

    public func uploadMedia(
        mediaUploadRequest: MediaUploadRequest
    ) async -> Result<WpNetworkResponse, MediaUploadRequestExecutionError> {
        (await perform(mediaUploadRequest))
            .mapError { error in
                switch error {
                case let .RequestExecutionFailed(statusCode, redirects, reason):
                    MediaUploadRequestExecutionError.RequestExecutionFailed(statusCode: statusCode, redirects: redirects, reason: reason)
                }
            }
    }

    func perform(_ request: NetworkRequestContent) async -> Result<WpNetworkResponse, RequestExecutionError> {
        do {
            var urlrequest = try request.buildURLRequest()

            // Set the user agent before `additionalHttpHeadersForAllRequests` so that it can be overridden that way
            urlrequest.setValue(self.userAgent, forHTTPHeaderField: "User-Agent")

            for (key, value) in additionalHttpHeadersForAllRequests {
                urlrequest.addValue(value, forHTTPHeaderField: key)
            }

            let (data, response) = try await self.fetch(request: urlrequest)

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

final class RequestExecutorDelegate: NSObject, URLSessionTaskDelegate, @unchecked Sendable {

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

extension URLRequest {
    var requestId: String? {
        allHTTPHeaderFields?["X-REQUEST-ID"]
    }
}

protocol NetworkRequestContent {
    func requestId() -> String
    func method() -> RequestMethod
    func url() -> WpEndpointUrl
    func headerMap() -> WpNetworkHeaderMap
    func encodeBody(into request: inout URLRequest) throws
}

extension NetworkRequestContent {
    func buildURLRequest() throws -> URLRequest {
        let url = URL(string: self.url())!
        var request = URLRequest(url: url)
        request.httpMethod = self.method().rawValue
        request.allHTTPHeaderFields = self.headerMap().toFlatMap()
        request.allHTTPHeaderFields?["X-REQUEST-ID"] = self.requestId()
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

}

extension MediaUploadRequest: NetworkRequestContent {

    func encodeBody(into request: inout URLRequest) throws {
        var form = [MultipartFormField]()
        for (name, value) in mediaParams() {
            form.append(.init(text: value, name: name))
        }
        try form.append(.init(fileAtPath: filePath(), name: "file"))

        let boundery = String(format: "wordpressrs.%08x", Int.random(in: Int.min..<Int.max))
        request.setValue("multipart/form-data; boundary=\(boundery)", forHTTPHeaderField: "Content-Type")
        request.httpBodyStream = try form
            .multipartFormDataStream(boundary: boundery, forceWriteToFile: false)
            .asInputStream()
    }

}
