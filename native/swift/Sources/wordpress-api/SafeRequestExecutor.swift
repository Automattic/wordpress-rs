import Foundation
import WordPressAPIInternal

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

public protocol SafeRequestExecutor: RequestExecutor, Sendable {
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
                return .success(
                    try WpNetworkResponse(
                        data: data,
                        request: request,
                        response: response
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
                        reason: .genericError(errorMessage: error.localizedDescription)
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
        abort() // TODO: This is implemented in a different branch, we'll sync it later
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

    // swiftlint:disable force_try
    func sleep(millis: UInt64) async {
        if #available(macOS 13.0, iOS 16.0, tvOS 16.0, watchOS 9.0, *) {
            try! await Task.sleep(for: .milliseconds(millis))
        } else {
            try! await Task.sleep(nanoseconds: millis * 1000)
        }
    }
    // swiftlint:enable force_try
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
