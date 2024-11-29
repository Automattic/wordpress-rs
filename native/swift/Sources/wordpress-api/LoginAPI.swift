import Foundation

#if canImport(WordPressAPIInternal)
import WordPressAPIInternal
#endif

#if os(Linux)
import FoundationNetworking
#endif

public final class WordPressLoginClient {

    public protocol AuthenticatorProtocol {
        func authenticate(url: URL, callbackURL: URL) async throws -> URL
    }

    private static let callbackURL = URL(string: "x-wordpress-app://login-callback")!

    public enum Error: Swift.Error {
        case invalidSiteAddress(UrlDiscoveryError)
        case missingLoginUrl
        case authenticationError(OAuthResponseUrlError)
        case invalidApplicationPasswordCallback
        case cancelled
        case unknown(Swift.Error)

        func isAutodiscoveryError() -> Bool {
            guard case let .invalidSiteAddress(urlDiscoveryError) = self else {
                return false
            }

            guard case .UrlDiscoveryFailed = urlDiscoveryError else {
                return false
            }

            return true
        }

        var isFailedToFetchApiDetails: Bool {
            guard
                case let .invalidSiteAddress(urlDiscoveryError) = self,
                case .UrlDiscoveryFailed(let attempts) = urlDiscoveryError
            else {
                return false
            }

            return attempts.values.contains { state in
                return switch state {
                case .failure(let error): urlDiscoverErrorIsFetchApiDetailsFailed(error)
                default: false
                }
            }
        }

        var isFailedToFetchApiRoot: Bool {
            guard
                case let .invalidSiteAddress(urlDiscoveryError) = self,
                case .UrlDiscoveryFailed(let attempts) = urlDiscoveryError
            else {
                return false
            }

            return attempts.values.contains { state in
                if case let .failure(error) = state {
                    return isFetchRootUrlFailedError(error)
                }

                return false
            }
        }

        private func urlDiscoverErrorIsFetchApiDetailsFailed(_ error: UrlDiscoveryAttemptError) -> Bool {
            return switch error {
            case .fetchApiDetailsFailed: true
            default: false
            }
        }

        private func isFetchRootUrlFailedError(_ error: UrlDiscoveryAttemptError) -> Bool {
            return switch error {
            case .fetchApiRootUrlFailed: true
            default: false
            }
        }
    }

    private let requestExecutor: SafeRequestExecutor
    private let client: UniffiWpLoginClient

    public convenience init(urlSession: URLSession) {
        self.init(requestExecutor: urlSession)
    }

    init(requestExecutor: SafeRequestExecutor) {
        self.requestExecutor = requestExecutor
        self.client = UniffiWpLoginClient(requestExecutor: requestExecutor)
    }

    public func login(
        site: String,
        appName: String,
        appId: WpUuid?,
        authenticator: AuthenticatorProtocol
    ) async throws -> WpApiApplicationPasswordDetails {
        let loginURL = try await self.loginURL(forSite: site)
        let authURL = createApplicationPasswordAuthenticationUrl(
            loginUrl: loginURL,
            appName: appName,
            appId: appId,
            successUrl: Self.callbackURL.absoluteString,
            rejectUrl: Self.callbackURL.absoluteString
        ).asURL()

        let url = try await authenticator.authenticate(url: authURL, callbackURL: Self.callbackURL)
        return try handleAuthenticationCallback(url)
    }

    private func loginURL(forSite proposedSiteUrl: String) async throws -> ParsedUrl {
        let result: UrlDiscoverySuccess
        do {
            result = try await client.apiDiscovery(siteUrl: proposedSiteUrl)
        } catch let error as UrlDiscoveryError {
            throw Error.invalidSiteAddress(error)
        } catch {
            throw Error.unknown(error)
        }

        // All sites should have some form of authentication we can use
        guard let passwordAuthenticationUrl = result.apiDetails.findApplicationPasswordsAuthenticationUrl(),
              let parsedLoginUrl = try? ParsedUrl.parse(input: passwordAuthenticationUrl) else {
            throw Error.missingLoginUrl
        }

        return parsedLoginUrl
    }

    private func handleAuthenticationCallback(
        _ urlWithToken: URL
    ) throws -> WpApiApplicationPasswordDetails {
        guard let parsed = try? ParsedUrl.from(url: urlWithToken) else {
            throw Error.invalidApplicationPasswordCallback
        }

        do {
            return try extractLoginDetailsFromUrl(url: parsed)
        } catch let error as OAuthResponseUrlError {
            throw Error.authenticationError(error)
        } catch {
            throw Error.unknown(error)
        }
    }
}

#if os(iOS) || os(macOS)

import AuthenticationServices

extension WordPressLoginClient {

    class AuthenticationServiceAuthenticator: NSObject,
                                              AuthenticatorProtocol,
                                              ASWebAuthenticationPresentationContextProviding {
        func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor {
            ASPresentationAnchor()
        }

        func authenticate(url: URL, callbackURL: URL) async throws -> URL {
            try await withCheckedThrowingContinuation { continuation in
                let session = ASWebAuthenticationSession(
                    url: url,
                    callbackURLScheme: "x-wordpress-app",
                    completionHandler: { url, error in
                        do {
                            continuation.resume(returning: try self.handleCompletion(url: url, error: error))
                        } catch {
                            continuation.resume(throwing: error)
                        }
                })
                session.presentationContextProvider = self
                session.start()
            }
        }

        private func handleCompletion(url: URL?, error: Swift.Error?) throws -> URL {
            if let url {
                return url
            } else if let error = error as? ASWebAuthenticationSessionError {
                switch error.code {
                case .canceledLogin:
                    throw Error.cancelled
                case .presentationContextInvalid, .presentationContextNotProvided:
                    assertionFailure("An unexpected error received: \(error)")
                    throw Error.cancelled
                @unknown default:
                    throw Error.cancelled
                }
            } else {
                throw Error.invalidApplicationPasswordCallback
            }
        }
    }

    public func login(
        site: String,
        appName: String,
        appId: WpUuid?
    ) async throws -> WpApiApplicationPasswordDetails {
        let provider = await AuthenticationServiceAuthenticator()
        return try await login(
            site: site,
            appName: appName,
            appId: appId,
            authenticator: provider
        )
    }
}

#endif
