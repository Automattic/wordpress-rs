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

                debugPrint(redirectTracker.redirects(for: request.requestId()))

                return .success(
                    WpNetworkResponse(
                        body: data,
                        statusCode: UInt16(httpResponse.statusCode),
                        headerMap: headerMap
                    )
                )
            } catch is WpNetworkHeaderMapError {
                return .failure(.RequestExecutionFailed(statusCode: nil, reason: "Invalid header"))
            } catch {
                return .failure(.RequestExecutionFailed(statusCode: nil, reason: "Unknown error: \(error)"))
            }
        } catch {
           if (try? errorIsHttpsError(error)) == true {
               guard let httpsErrorDetails = try? getHttpsErrorCertificateDetails(error) else {
                   abort()
               }

               return .failure(.RequestExecutionFailed(statusCode: nil, reason: httpsErrorDetails.description))
           }

           return .failure(.RequestExecutionFailed(statusCode: nil, reason: error.localizedDescription))
       }
    }

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

    struct Redirect {
        var source: URL
        var destination: URL
    }

    private let lock = NSLock()
    private var redirects: [String: [Redirect]] = [:]

    func redirects(for taskID: String) -> [Redirect]? {
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
                redirects[requestID] = [Redirect]()
            }

            redirects[requestID]?.append(Redirect(
                source: source,
                destination: destination
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
