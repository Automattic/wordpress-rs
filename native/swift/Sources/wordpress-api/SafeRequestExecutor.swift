import Foundation
import WordPressAPIInternal
import X509

#if os(Linux)
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
               guard let httpsErrorDetails = try? getHttpsErrorCertificateDetails(error) else {
                   abort()
               }

               let domain = httpsErrorDetails.first?.commonName ?? "Unknown Domain"

               return .failure(.RequestExecutionFailed(
                    statusCode: nil,
                    redirects: nil,
                    reason: .sslError(
                        domain: domain,
                        trustChain: nil,
                        errorMessage: error.localizedDescription
                    )
                ))
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

    private func getHttpsErrorCertificateDetails(_ error: Error) throws -> [SSLCertificateInfo] {
        let nserror = error as NSError
        let info = nserror.userInfo

        return try parseCertificateChain(info["NSErrorPeerCertificateChainKey"] as! NSArray)
            .compactMap(self.parseCertificateItem)
    }

    func parseCertificateItem(_ certificate: Certificate) throws -> SSLCertificateInfo? {
        let validDomainNames = try? certificate
            .extensions
            .subjectAlternativeNames?
            .compactMap { generalName -> String? in
                switch generalName {
                case .dnsName(let domain): return domain
                default: return nil
                }
            }

        return SSLCertificateInfo(
            commonName: certificate.subject.asSSLCertificateSubject.commonName ?? "no common name found",
            validDomainNames: validDomainNames ?? [],
            issuer: certificate.issuer.asSSLCertificateSubject
        )
    }

    struct SSLCertificateInfo {
        // The domain this certificate was issued for
        let commonName: String

        // The list of valid domain names from the SAN field of the certificate – many certificates are valid for
        // multiple domain names
        let validDomainNames: [String]

        // The chain of trust back to the root cert
        let issuer: SSLCertificateSubject
    }

    struct SSLCertificateSubject {
        let region: String?
        let organization: String?
        let commonName: String?
    }

    private func parseCertificateChain(_ chain: NSArray) throws -> [Certificate] {
        try chain.compactMap { cert in

            // CFGetTypeID validates the type in a way the type system can't
            let typeCert = cert as! SecCertificate
            guard CFGetTypeID(typeCert) == SecCertificateGetTypeID() else {
                return nil
            }

            let certData = SecCertificateCopyData(cert as! SecCertificate) as Data
            return try Certificate(derEncoded: [UInt8](certData))
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

extension DistinguishedName {
    fileprivate var asSSLCertificateSubject: WpRequestExecutor.SSLCertificateSubject {

        if self.count == 1 {
            return WpRequestExecutor
                .SSLCertificateSubject(
                    region: nil,
                    organization: nil,
                    commonName: self.first?.first?.value.description
                )
        }

        if self.count == 3 {
            return WpRequestExecutor.SSLCertificateSubject(
                region: self[0].first?.value.description,
                organization: self[1].first?.value.description,
                commonName: self[2].first?.value.description
            )
        }

        abort()
    }
}
