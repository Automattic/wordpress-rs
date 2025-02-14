import Foundation
import WordPressAPIInternal

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

public protocol SafeRequestExecutor: RequestExecutor, Sendable {
    func withCredential(_ credential: URLCredential) -> Self

    func execute(_ request: WpNetworkRequest) async -> Result<WpNetworkResponse, RequestExecutionError>
}

extension SafeRequestExecutor {
    public func execute(request: WpNetworkRequest) async throws -> WpNetworkResponse {
        let result = await execute(request)
        return try result.get()
    }
}

final class WpRequestExecutor: SafeRequestExecutor {
    private let session: URLSession
    private let executorDelegate: RequestExecutorDelegate

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

    // swiftlint:disable:next cyclomatic_complexity
    func execute(_ request: WpNetworkRequest) async -> Result<WpNetworkResponse, RequestExecutionError> {

        let (data, response): (Data, URLResponse)

        do {
            var urlrequest = request.asURLRequest()

            for (key, value) in additionalHttpHeadersForAllRequests {
                urlrequest.addValue(value, forHTTPHeaderField: key)
            }

            (data, response) = try await self.session.data(for: urlrequest, delegate: executorDelegate)

            guard let httpResponse = response as? HTTPURLResponse else {
                preconditionFailure("The HTTP response should always be a HTTPURLResponse")
            }

            let headerMap: WpNetworkHeaderMap

            do {
                headerMap = try WpNetworkHeaderMap.fromMap(hashMap: httpResponse.httpHeaders)

                if isHttpAuthenticationInvalidResponse(httpResponse, for: request) {
                    return handleHttpAuthenticationInvalidResponse(httpResponse, for: request)
                }

                if isHttpAuthenticationRequiredResponse(httpResponse) {
                    return handleHttpAuthenticationRequiredResponse(httpResponse, for: request)
                }

                if isRateLimitExceededResponse(httpResponse) {
                    return try await handleRateLimitExceededResponse(httpResponse, for: request)
                }

                return .success(
                    WpNetworkResponse(
                        body: data,
                        statusCode: UInt16(httpResponse.statusCode),
                        headerMap: headerMap
                    )
                )
            } catch is WpNetworkHeaderMapError {
                let error = RequestExecutionError.RequestExecutionFailed(
                    statusCode: nil,
                    redirects: executorDelegate.redirects(for: request.requestId()),
                    reason: RequestExecutionErrorReason.genericError(errorMessage: "Invalid Headers")
                )

                return .failure(error)

            } catch {
                return .failure(
                    .RequestExecutionFailed(
                        statusCode: nil,
                        redirects: executorDelegate.redirects(for: request.requestId()),
                        reason: .genericError(errorMessage: "Unknown error: \(error)")
                    )
                )
            }
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

    private func handleHttpsError(
        _ error: Error,
        for request: WpNetworkRequest
    ) -> Result<WpNetworkResponse, RequestExecutionError> {

        guard
            var peerCertificateChain = getPeerCertificateChain(error),
            !peerCertificateChain.isEmpty
        else {
            return .failure(.RequestExecutionFailed(
                 statusCode: nil,
                 redirects: executorDelegate.redirects(for: request.requestId()),
                 reason: .invalidSslError(
                     siteCertificate: nil,
                     certificateChain: [],
                     errorMessage: error.localizedDescription,
                     suggestedAction: (error as NSError).localizedRecoverySuggestion
                 )
            ))
        }

        let siteCertificate = peerCertificateChain.remove(at: 0)

        return .failure(
         .RequestExecutionFailed(
             statusCode: nil,
             redirects: executorDelegate.redirects(for: request.requestId()),
             reason: RequestExecutionErrorReason.invalidSslError(
                     siteCertificate: siteCertificate,
                     certificateChain: peerCertificateChain,
                     errorMessage: error.localizedDescription,
                     suggestedAction: (error as NSError).localizedRecoverySuggestion
                 )
             )
         )
    }

    func handleHttpAuthenticationRequiredResponse(
        _ response: HTTPURLResponse,
        for request: WpNetworkRequest
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        .failure(
            .RequestExecutionFailed(
                statusCode: UInt16(response.statusCode),
                redirects: executorDelegate.redirects(for: request.requestId()),
                reason: .httpAuthenticationRequiredError(
                    url: request.url(),
                    serverMessage: response.value(forHTTPHeaderField: "WWW-Authenticate")
                )
            )
        )
    }

    func handleHttpAuthenticationInvalidResponse(
        _ response: HTTPURLResponse,
        for request: WpNetworkRequest
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        .failure(
            .RequestExecutionFailed(
                statusCode: UInt16(response.statusCode),
                redirects: executorDelegate.redirects(for: request.requestId()),
                reason: .httpAuthenticationRejectedError(
                    url: request.url(),
                    serverMessage: response.value(forHTTPHeaderField: "WWW-Authenticate")
                )
            )
        )
    }

    func handleNonExistentSiteError(
        _ error: Error,
        for request: WpNetworkRequest
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

    func uploadMedia(mediaUploadRequest: MediaUploadRequest) async throws -> WpNetworkResponse {
        try WpNetworkResponse(body: Data(), statusCode: 500, headerMap: .fromMap(hashMap: [:]))
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

    private func isRateLimitExceededResponse(_ response: HTTPURLResponse) -> Bool {
        response.statusCode == 429
    }

    private func handleRateLimitExceededResponse(
        _ response: HTTPURLResponse,
        for request: WpNetworkRequest
    ) async throws -> Result<WpNetworkResponse, RequestExecutionError> {

        let defaultWaitTime: TimeInterval = 5

        func wait(for timeInterval: TimeInterval) async throws {
            if #available(macOS 13.0, iOS 16.0, watchOS 9.0, tvOS 16.0, *) {
                try await Task.sleep(for: .seconds(timeInterval))
            } else {
                try await Task.sleep(nanoseconds: UInt64(timeInterval) * 1_000_000_000)
            }
        }

        let newRequest = request.incrementRetryCount()

        guard
            let retryAfterString = response.allHeaderFields["retry-after"] as? String,
            let retryAfterDuration = TimeInterval.fromRetryHeaderValue(retryAfterString)
        else {
            try await wait(for: defaultWaitTime)
            return await execute(newRequest)
        }

        try await wait(for: retryAfterDuration)

        if newRequest.retryCount() >= 2 {
            return .failure(
                .RequestExecutionFailed(
                    statusCode: UInt16(response.statusCode),
                    redirects: executorDelegate.redirects(for: request.requestId()),
                    reason: .misconfiguredRateLimitError
                )
            )
        }

        return await execute(newRequest)
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

final class RequestExecutorDelegate: NSObject, URLSessionTaskDelegate, @unchecked Sendable {

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

    func urlSession(
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
