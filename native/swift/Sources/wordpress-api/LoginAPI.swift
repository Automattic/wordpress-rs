import Foundation
import WordPressAPIInternal

#if os(Linux)
import FoundationNetworking
#endif

public final class WordPressLoginClient {

    public protocol AuthenticatorProtocol {
        func authenticate(url: URL, callbackURL: URL) async throws(WordPressLoginClientError) -> URL
    }

    private static let callbackURL = URL(string: "x-wordpress-app://login-callback")!

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
    ) async throws(WordPressLoginClientError) -> WpApiApplicationPasswordDetails {
        let loginURL = try await self.loginURL(forSite: site)
        let authURL = createApplicationPasswordAuthenticationUrl(
            loginUrl: loginURL,
            appName: appName,
            appId: appId,
            successUrl: Self.callbackURL.absoluteString,
            rejectUrl: Self.callbackURL.absoluteString
        )
        .asURL()

        let urlWithToken = try await authenticator.authenticate(url: authURL, callbackURL: Self.callbackURL)
        return try handleAuthenticationCallback(urlWithToken)
    }

    private func loginURL(forSite proposedSiteUrl: String) async throws(WordPressLoginClientError) -> ParsedUrl {

        do {
            let client = UniffiWpLoginClient(requestExecutor: self.requestExecutor)
            let discoveryResult = try await client.apiDiscovery(siteUrl: proposedSiteUrl)

            // All sites should have some form of authentication we can use
            guard
                let passwordAuthenticationUrl = discoveryResult.apiDetails.findApplicationPasswordsAuthenticationUrl(),
                let parsedLoginUrl = try? ParsedUrl.parse(input: passwordAuthenticationUrl)
            else {
                throw WordPressLoginClientError.missingLoginUrl
            }

            return parsedLoginUrl

        } catch let error as UrlDiscoveryError {
            throw WordPressLoginClientError.invalidSiteAddress(error)
        } catch {
            throw WordPressLoginClientError.unknown(error)
        }
    }

    private func handleAuthenticationCallback(
        _ urlWithToken: URL
    ) throws(WordPressLoginClientError) -> WpApiApplicationPasswordDetails {
        guard let parsed = try? ParsedUrl.from(url: urlWithToken) else {
            throw .invalidApplicationPasswordCallback
        }

        do {
            return try extractLoginDetailsFromUrl(url: parsed)
        } catch let error as OAuthResponseUrlError {
            throw .authenticationError(error)
        } catch {
            throw .unknown(error)
        }
    }
}

#if os(iOS) || os(macOS)

import AuthenticationServices

extension WordPressLoginClient {

    class AuthenticationServiceAuthenticator: NSObject, AuthenticatorProtocol,
                                                ASWebAuthenticationPresentationContextProviding {
        func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor {
            ASPresentationAnchor()
        }

        @MainActor
        func authenticate(url: URL, callbackURL: URL) async throws(WordPressLoginClientError) -> URL {
            do {
                return try await withCheckedThrowingContinuation { continuation in
                    let session = ASWebAuthenticationSession(
                        url: url,
                        callbackURLScheme: callbackURL.scheme!
                    ) { url, error in
                        if let url {
                            continuation.resume(returning: url)
                        } else if let error = error as? ASWebAuthenticationSessionError {
                            switch error.code {
                            case .canceledLogin:
                                continuation.resume(throwing: WordPressLoginClientError.cancelled)
                            case .presentationContextInvalid, .presentationContextNotProvided:
                                assertionFailure("An unexpected error received: \(error)")
                                continuation.resume(throwing: WordPressLoginClientError.cancelled)
                            @unknown default:
                                continuation.resume(throwing: WordPressLoginClientError.cancelled)
                            }
                        } else {
                            continuation.resume(throwing: WordPressLoginClientError.invalidApplicationPasswordCallback)
                        }
                    }
                    session.presentationContextProvider = self
                    session.start()
                }
            } catch {
                // swiftlint:disable:next force_cast
                throw error as! WordPressLoginClientError
            }
        }
    }

    public func login(
        site: String,
        appName: String,
        appId: WpUuid?
    ) async throws(WordPressLoginClientError) -> WpApiApplicationPasswordDetails {
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
