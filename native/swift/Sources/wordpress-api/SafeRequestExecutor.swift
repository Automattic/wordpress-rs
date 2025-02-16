import Foundation
import WordPressAPIInternal

#if os(Linux)
import FoundationNetworking
#endif

extension URLSession {
    public func asRequestExecutor() -> some RequestExecutor {
        WpRequestExecutor(urlSession: self)
    }
}

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
    private let redirectTracker = RedirectTracker()

    init(urlSession: URLSession) {
        self.session = urlSession
    }

    // swiftlint:disable function_body_length
    func execute(_ request: WpNetworkRequest) async -> Result<WpNetworkResponse, RequestExecutionError> {

        let (data, response): (Data, URLResponse)

        do {
            let urlrequest = request.asURLRequest()
            (data, response) = try await self.session.data(for: urlrequest, delegate: redirectTracker)

            // swiftlint:disable:next force_cast
            let httpResponse = response as! HTTPURLResponse

            let headerMap: WpNetworkHeaderMap

            do {
                headerMap = try WpNetworkHeaderMap.fromMap(hashMap: httpResponse.httpHeaders)

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
                    redirects: redirectTracker.redirects(for: request.requestId()),
                    reason: RequestExecutionErrorReason.genericError(errorMessage: "Invalid Headers")
                )

                return .failure(error)

            } catch {
                return .failure(
                    .RequestExecutionFailed(
                        statusCode: nil,
                        redirects: redirectTracker.redirects(for: request.requestId()),
                        reason: .genericError(errorMessage: "Unknown error: \(error)")
                    )
                )
            }
        } catch {
           if (try? errorIsHttpsError(error)) == true {
               guard var peerCertificateChain = try? getPeerCertificateChain(error) else {
                   abort() // TODO
               }

               if peerCertificateChain.isEmpty {
                   return .failure(
                    .RequestExecutionFailed(
                        statusCode: nil,
                        redirects: redirectTracker.redirects(for: request.requestId()),
                        reason: .invalidSslError(
                            siteCertificate: nil,
                            certificateChain: [],
                            errorMessage: error.localizedDescription,
                            suggestedAction: (error as NSError).localizedRecoverySuggestion
                        )
                    )
                   )
               }

               let siteCertificate = peerCertificateChain.remove(at: 0)

               return .failure(
                .RequestExecutionFailed(
                    statusCode: nil,
                    redirects: redirectTracker.redirects(for: request.requestId()),
                    reason: RequestExecutionErrorReason.invalidSslError(
                            siteCertificate: siteCertificate,
                            certificateChain: peerCertificateChain,
                            errorMessage: nil,
                            suggestedAction: nil
                        )
                    )
                )
           }

            return .failure(.RequestExecutionFailed(
                statusCode: nil,
                redirects: nil,
                reason: .genericError(errorMessage: error.localizedDescription)
            ))
       }
    }
    // swiftlint:enable function_body_length

    func uploadMedia(mediaUploadRequest: MediaUploadRequest) async throws -> WpNetworkResponse {
        try WpNetworkResponse(body: Data(), statusCode: 500, headerMap: .fromMap(hashMap: [:]))
    }

    // swiftlint:disable force_cast
    private func errorIsHttpsError(_ error: Error) throws -> Bool {
        let nserror = error as NSError

        return nserror.domain == NSURLErrorDomain && [
            NSURLErrorServerCertificateUntrusted,
            NSURLErrorSecureConnectionFailed,
            NSURLErrorServerCertificateHasBadDate,
            NSURLErrorServerCertificateNotYetValid
        ].contains(nserror.code)
    }

    private func getPeerCertificateChain(_ error: Error) throws -> [SSLCertificateInfo] {
        let nserror = error as NSError
        let info = nserror.userInfo

        guard let certChainArray = info["NSErrorPeerCertificateChainKey"] as? NSArray else {
            return []
        }

        return try parseCertificateChain(certChainArray).compactMap { data in
            parseCertificate(data: data)
        }
    }

    private func parseCertificateChain(_ chain: NSArray) throws -> [Data] {
        chain.compactMap { cert in

            #if os(Linux)
            return nil
            #else
            // CFGetTypeID validates the type in a way the type system can't
            let typeCert = cert as! SecCertificate
            guard CFGetTypeID(typeCert) == SecCertificateGetTypeID() else {
                return nil
            }

            return SecCertificateCopyData(cert as! SecCertificate) as Data
            #endif
        }
    }
    // swiftlint:enable force_cast
}

final class RedirectTracker: NSObject, URLSessionTaskDelegate, @unchecked Sendable {

    private let lock = NSLock()
    private var redirects: [String: [WpRedirect]] = [:]

    func redirects(for taskID: String) -> [WpRedirect]? {
        lock.withLock {
            redirects[taskID]
        }
    }

    func removeRedirects(for taskID: String) {
        lock.withLock {
            redirects[taskID] = nil
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
