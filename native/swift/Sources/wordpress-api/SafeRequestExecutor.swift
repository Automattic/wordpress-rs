import Foundation
import WordPressAPIInternal

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

public protocol SafeRequestExecutor: RequestExecutor, Sendable {
    var executorDelegate: RequestExecutorDelegate { get }
    func withCredential(_ credential: URLCredential) -> Self

    func execute(_ request: WpNetworkRequest) async -> Result<WpNetworkResponse, RequestExecutionError>
    func executeRaw(_ request: URLRequest) async throws -> (Data, HTTPURLResponse)
}

extension SafeRequestExecutor {
    public func execute(request: WpNetworkRequest) async throws -> WpNetworkResponse {
        let result = await execute(request)
        return try result.get()
    }

    func handleAutomaticRetryIfNeeded(
        with data: Data,
        for response: HTTPURLResponse,
        to request: URLRequest
    ) async throws -> (Data, HTTPURLResponse) {

        if isRateLimitExceededResponse(response) {
            return try await handleRateLimitExceededResponse(response, for: request)
        }

        return (data, response)
    }

    func processRawResponse(
        _ response: HTTPURLResponse,
        for request: WpNetworkRequest,
        with data: Data
    ) async throws -> Result<WpNetworkResponse, RequestExecutionError> {

        let headerMap = try WpNetworkHeaderMap.fromMap(hashMap: response.httpHeaders)

        if isHttpAuthenticationInvalidResponse(response, for: request) {
            return .failure(handleHttpAuthenticationInvalidResponse(response, for: request))
        }

        if isHttpAuthenticationRequiredResponse(response) {
            return .failure(handleHttpAuthenticationRequiredResponse(response, for: request))
        }

        return .success(
            WpNetworkResponse(
                body: data,
                statusCode: UInt16(response.statusCode),
                headerMap: headerMap
            )
        )

    }

    func processTransportError(
        _ error: Error,
        for request: WpNetworkRequest
    ) -> RequestExecutionError {

        if errorIsHttpsError(error) {
            return handleHttpsError(error, for: request)
        }

        if errorIsNonExistentSiteError(error) {
            return handleNonExistentSiteError(error, for: request)
        }

        if errorIsDeviceIsOffline(error) {
            return handleDeviceIsOfflineError(error)
        }

        return .RequestExecutionFailed(
            statusCode: nil,
            redirects: nil,
            reason: .genericError(errorMessage: error.localizedDescription)
        )
    }

    func isRateLimitExceededResponse(_ response: WpNetworkResponse) -> Bool {
        response.statusCode == 429
    }

    func isRateLimitExceededResponse(_ response: HTTPURLResponse) -> Bool {
        response.statusCode == 429
    }

    func handleRateLimitExceededResponse(
        _ response: HTTPURLResponse,
        for request: URLRequest
    ) async throws -> (Data, HTTPURLResponse) {

        var response = response

        let defaultWaitTime: TimeInterval = 5

        func wait(for timeInterval: TimeInterval) async throws {
            if #available(macOS 13.0, iOS 16.0, watchOS 9.0, tvOS 16.0, *) {
                try await Task.sleep(for: .seconds(timeInterval))
            } else {
                try await Task.sleep(nanoseconds: UInt64(timeInterval) * 1_000_000_000)
            }
        }

        var data = Data()

        for _ in 1...5 {
            let retryAfterDuration = response.retryAfter ?? defaultWaitTime
            try await wait(for: retryAfterDuration)

            let (newData, newResponse) = try await executeRaw(request)

            if !isRateLimitExceededResponse(newResponse) {
                return (newData, newResponse)
            }

            data = newData
            response = newResponse
        }

        return (data, response)
    }

    private func handleHttpsError(
        _ error: Error,
        for request: WpNetworkRequest
    ) -> RequestExecutionError {

        guard
            var peerCertificateChain = getPeerCertificateChain(error),
            !peerCertificateChain.isEmpty
        else {
            return .RequestExecutionFailed(
                 statusCode: nil,
                 redirects: executorDelegate.redirects(for: request.requestId()),
                 reason: .invalidSslError(
                     siteCertificate: nil,
                     certificateChain: [],
                     errorMessage: error.localizedDescription,
                     suggestedAction: (error as NSError).localizedRecoverySuggestion
                 )
            )
        }

        let siteCertificate = peerCertificateChain.remove(at: 0)

        return .RequestExecutionFailed(
             statusCode: nil,
             redirects: executorDelegate.redirects(for: request.requestId()),
             reason: RequestExecutionErrorReason.invalidSslError(
                     siteCertificate: siteCertificate,
                     certificateChain: peerCertificateChain,
                     errorMessage: error.localizedDescription,
                     suggestedAction: (error as NSError).localizedRecoverySuggestion
                 )
            )
    }

    func handleHttpAuthenticationRequiredResponse(
        _ response: HTTPURLResponse,
        for request: WpNetworkRequest
    ) -> RequestExecutionError {
        .RequestExecutionFailed(
            statusCode: UInt16(response.statusCode),
            redirects: executorDelegate.redirects(for: request.requestId()),
            reason: .httpAuthenticationRequiredError(
                url: request.url(),
                serverMessage: response.value(forHTTPHeaderField: "WWW-Authenticate")
            )
        )
    }

    func handleHttpAuthenticationInvalidResponse(
        _ response: HTTPURLResponse,
        for request: WpNetworkRequest
    ) -> RequestExecutionError {
        .RequestExecutionFailed(
            statusCode: UInt16(response.statusCode),
            redirects: executorDelegate.redirects(for: request.requestId()),
            reason: .httpAuthenticationRejectedError(
                url: request.url(),
                serverMessage: response.value(forHTTPHeaderField: "WWW-Authenticate")
            )
        )
    }

    func handleNonExistentSiteError(
        _ error: Error,
        for request: WpNetworkRequest
    ) -> RequestExecutionError {
        .RequestExecutionFailed(
            statusCode: nil,
            redirects: executorDelegate.redirects(for: request.requestId()),
            reason: .nonExistentSiteError(
                errorMessage: error.localizedDescription,
                suggestedAction: (error as NSError).localizedRecoverySuggestion
            )
        )
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

    /// This response indicates that HTTP credentials must be sent as part of the request
    private func isHttpAuthenticationRequiredResponse(_ response: HTTPURLResponse) -> Bool {
        response.statusCode == 401
    }

    /// This response indicates that the provided credentials are invalid (or the user doesn't have access to this)
    private func isHttpAuthenticationInvalidResponse(
        _ response: HTTPURLResponse,
        for request: WpNetworkRequest
    ) -> Bool {

        // It's hard to get a good signal that an error 403 occcured, but if the server returns it directly
        // we'll pass that along
        if response.statusCode == 403 {
            return true
        }

        return executorDelegate.isHttp403Failed(for: request.requestId())
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
    ) -> RequestExecutionError {
        .RequestExecutionFailed(
            statusCode: nil,
            redirects: nil,
            reason: .deviceIsOfflineError(
                errorMessage: error.localizedDescription
            )
        )
    }

    private func getPeerCertificateChain(_ error: Error) -> [SSLCertificateInfo]? {
        let nserror = error as NSError
        let info = nserror.userInfo

        guard let certChainArray = info["NSErrorPeerCertificateChainKey"] as? NSArray else {
            return nil
        }

        return parseCertificateChain(certChainArray).compactMap { data in
            parseCertificate(data: data)
        }
    }

    // swiftlint:disable force_cast
    private func parseCertificateChain(_ chain: NSArray) -> [Data] {
        #if os(Linux) // Linux doesn't know about the types here, so we'll fast-path our way out of it
        return []
        #else
        return chain.compactMap { cert in

            // CFGetTypeID validates the type in a way the type system can't
            let typeCert = cert as! SecCertificate
            guard CFGetTypeID(typeCert) == SecCertificateGetTypeID() else {
                return nil
            }

            return SecCertificateCopyData(cert as! SecCertificate) as Data
        }
        #endif
    }
    // swiftlint:enable force_cast
}

final class WpRequestExecutor: SafeRequestExecutor {
    private let session: URLSession
    let executorDelegate: RequestExecutorDelegate

    private let additionalHttpHeadersForAllRequests: [String: String]

    init(
        urlSession: URLSession,
        httpCredential: URLCredential? = nil,
        additionalHttpHeadersForAllRequests: [String: String] = [:]
    ) {
        self.session = urlSession
        self.executorDelegate = RequestExecutorDelegate(credential: httpCredential)
        self.additionalHttpHeadersForAllRequests = additionalHttpHeadersForAllRequests
    }

    func withCredential(_ credential: URLCredential) -> WpRequestExecutor {
        WpRequestExecutor(urlSession: self.session, httpCredential: credential)
    }

    func executeRaw(_ request: URLRequest) async throws -> (Data, HTTPURLResponse) {
        var urlrequest = request

        for (key, value) in additionalHttpHeadersForAllRequests {
            urlrequest.addValue(value, forHTTPHeaderField: key)
        }

        let (data, response) = try await self.session.data(for: urlrequest, delegate: executorDelegate)

        guard let httpResponse = response as? HTTPURLResponse else {
            preconditionFailure("The HTTP response should always be a HTTPURLResponse")
        }

        return (data, httpResponse)
    }

    func execute(_ request: URLRequest) async -> Result<WpNetworkResponse, RequestExecutionError> {

        do {
            let (data, httpResponse) = try await executeRaw(request)
            try await handleAutomaticRetryIfNeeded(for: httpResponse, to: request)
            return try await processRawResponse(httpResponse, for: request, with: data)
        } catch {
            return .failure(processTransportError(error, for: request))
        }
    }

    func uploadMedia(mediaUploadRequest: MediaUploadRequest) async throws -> WpNetworkResponse {
        try WpNetworkResponse(body: Data(), statusCode: 500, headerMap: .fromMap(hashMap: [:]))
    }
}

public final class RequestExecutorDelegate: NSObject, URLSessionTaskDelegate, @unchecked Sendable {

    private let lock = NSLock()
    private var redirects: [String: [WpRedirect]] = [:]
    private var http403Failures: Set<String> = []

    let credential: URLCredential?

    init(redirects: [String: [WpRedirect]] = [:], credential: URLCredential? = nil) {
        self.redirects = redirects
        self.credential = credential
    }

    func redirects(for taskID: String) -> [WpRedirect]? {
        lock.withLock {
            redirects[taskID]
        }
    }

    func isHttp403Failed(for taskID: String) -> Bool {
        lock.withLock {
            http403Failures.contains(taskID)
        }
    }

    public func urlSession(
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

    public func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        didReceive challenge: URLAuthenticationChallenge) async
    -> (URLSession.AuthChallengeDisposition, URLCredential?) {

        let authMethod = challenge.protectionSpace.authenticationMethod

        guard authMethod == NSURLAuthenticationMethodHTTPBasic else {
            return (.performDefaultHandling, nil)
        }

        // Only try the credential once
        if challenge.previousFailureCount > 0 {
            if let requestID = task.originalRequest?.requestId {
                self.http403Failures.insert(requestID)
            }

            return (.performDefaultHandling, nil)
        }

        return (.useCredential, credential)
    }
}

extension URLRequest {
    var requestId: String? {
        allHTTPHeaderFields?["X-REQUEST-ID"]
    }
}
