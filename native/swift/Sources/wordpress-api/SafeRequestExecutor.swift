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
        do {
            let (data, response) = try await self.fetch(request: self.preflight(request.asURLRequest()))
            return .success(try WpNetworkResponse(data: data, request: request, response: response))
        } catch {
            return self.handleRequestError(error, request: request)
        }
    }

    public func uploadMedia(mediaUploadRequest: MediaUploadRequest) async throws -> WpNetworkResponse {
        let urlrequest = try await mediaUploadRequest.asUrlRequest()
        let (_, response) = try await self.fetch(request: self.preflight(urlrequest))
        return try WpNetworkResponse(mediaUploadRequest: mediaUploadRequest, response: response)
    }

    private func fetch(request: URLRequest) async throws -> (Data, HTTPURLResponse) {
        #if os(Linux)
        let (data, response) = try await session.data(for: request)
        guard let httpResponse = response as? HTTPURLResponse else {
            preconditionFailure("Unable to convert URLSession response to HTTPURLResponse")
        }
        return (data, httpResponse)
        #else
        let (data, response) = try await session.data(for: request, delegate: executorDelegate)
        guard let httpResponse = response as? HTTPURLResponse else {
            preconditionFailure("Unable to convert URLSession response to HTTPURLResponse")
        }
        return (data, httpResponse)
        #endif
    }

    private func handleRequestError(
        _ error: Error,
        request: WpNetworkRequest
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
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
                 reason: .invalidSslError(reason: InvalidSslErrorReason.genericSslError)
            ))
        }

        let siteCertificate = peerCertificateChain.remove(at: 0)

        return .failure(.RequestExecutionFailed(
             statusCode: nil,
             redirects: executorDelegate.redirects(for: request.requestId()),
             reason: RequestExecutionErrorReason.invalidSslError(
                reason: .certificateNotValidForName(
                    hostname: request.asURLRequest().url?.host ?? "unknown host",
                    presentedHostnames: [siteCertificate.commonName()]
                )
             )
         ))
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

    private func preflight(_ request: URLRequest) -> URLRequest {
        var mutableCopy = request

        // Set the user agent before `additionalHttpHeadersForAllRequests` so that it can be overridden that way
        mutableCopy.setValue(self.userAgent, forHTTPHeaderField: "User-Agent")

        for (key, value) in additionalHttpHeadersForAllRequests {
            mutableCopy.addValue(value, forHTTPHeaderField: key)
        }

        return mutableCopy
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
