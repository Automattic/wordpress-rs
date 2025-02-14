import Foundation
import WordPressAPIInternal

#if os(Linux)
import FoundationNetworking
#endif

public enum LoginError: LocalizedError {
    case missingApplicationPasswordAuthMethod(WpApiDetails)
    case `internal`(String?)
    case notAWordPressSite(String)

    // swiftlint:disable line_length
    public var errorDescription: String? {
        switch self {
        case .missingApplicationPasswordAuthMethod(let apiDetails):
            if apiDetails.usesHttps() {
                if apiDetails.hasApplicationPasswordBlockingPlugin() {
                    let blockingPlugins = apiDetails.applicationPasswordBlockingPlugins()

                    if blockingPlugins.count == 1 {
                        return "Unable to login to \(apiDetails.siteUrlString()) – the \(blockingPlugins.first!.name) plugin might have disabled Application Passwords. Please visit \(blockingPlugins.first!.supportUrl) to learn more."
                    } else {
                        return "Unable to login to \(apiDetails.siteUrlString()) – there are multiple installed plugins that might have disabled Application Passwords. Please disable them and try again."
                    }
                } else {
                    return "Application Passwords is not enabled for this site."
                }
            } else {
                if apiDetails.siteUrlIsLocalDevelopmentEnvironment() {
                    return "This site is a local development environment. You'll need to enable application passwords to connect to it with the app."
                }

                return "Application Passwords is not enabled for this site – this is likely because we can't establish a secure connection to it. Please add an SSL certificate to this site and try again."
            }

        case .internal(let underlyingErrorMessage):
            return underlyingErrorMessage

        case .notAWordPressSite(let urlString):
            return "Unable to login to \(urlString). Please double-check that this is a WordPress site"
        }
    }
    // swiftlint:enable line_length
}

public final class WordPressLoginClient {

    public protocol AuthenticatorProtocol {
        func authenticate(url: URL, callbackURL: URL) async throws -> URL
    }

    private static let callbackURL = URL(string: "x-wordpress-app://login-callback")!

    private let requestExecutor: SafeRequestExecutor
    private let client: UniffiWpLoginClient

    public convenience init(urlSession: URLSession) {
        self.init(requestExecutor: WpRequestExecutor(urlSession: urlSession))
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
        )
        .asURL()

        let urlWithToken = try await authenticator.authenticate(url: authURL, callbackURL: Self.callbackURL)
        return try handleAuthenticationCallback(urlWithToken)
    }

    public func loginAttempt(
        forSite proposedSiteUrl: String,
        credential: URLCredential? = nil
    ) async throws(LoginError) -> AutoDiscoveryAttemptResult {

        let temporaryExecutor: SafeRequestExecutor

        if let credential {
            temporaryExecutor = self.requestExecutor.withCredential(credential)
        } else {
            temporaryExecutor = self.requestExecutor
        }

        let client = UniffiWpLoginClient(requestExecutor: temporaryExecutor)
        let discoveryResult = await client.apiDiscovery(siteUrl: proposedSiteUrl)

        guard let successfulAttempt = discoveryResult.successfulAttempt else {
            if discoveryResult.userInputAttempt.isWordpressSite() {
                throw LoginError.internal(discoveryResult.userInputAttempt.errorMessage())
            } else {
                throw LoginError.notAWordPressSite(discoveryResult.userInputAttempt.attemptSiteUrl())
            }
        }

        return successfulAttempt
    }

    public func loginURL(
        forSite proposedSiteUrl: String,
        credential: URLCredential? = nil
    ) async throws(LoginError) -> ParsedUrl {

        // All sites should have some form of authentication we can use
        guard let apiDetails = try await loginAttempt(
            forSite: proposedSiteUrl,
            credential: credential
        ).apiDetails() else {
            preconditionFailure("It shouldn't be possible to hit this case – an error should be emitted instead")
        }

        guard
            let passwordAuthenticationUrl = apiDetails.findApplicationPasswordsAuthenticationUrl(),
            let parsedLoginUrl = try? ParsedUrl.parse(input: passwordAuthenticationUrl)
        else {
            throw LoginError.missingApplicationPasswordAuthMethod(apiDetails)
        }

        return parsedLoginUrl
    }

    private func handleAuthenticationCallback(
        _ urlWithToken: URL
    ) throws -> WpApiApplicationPasswordDetails {
//        guard let parsed = try? ParsedUrl.from(url: urlWithToken) else {
//            throw .invalidApplicationPasswordCallback
//        }

//        do {
//            return try extractLoginDetailsFromUrl(url: parsed)
//        } catch let error as OAuthResponseUrlError {
//            throw .authenticationError(error)
//        } catch {
//            throw .unknown(error)
//        }

        abort()
    }
}

#if os(iOS) || os(macOS) || os(tvOS) || os(watchOS)
import AuthenticationServices

extension WordPressLoginClient {

    class AuthenticationServicesAuthenticator: NSObject, AuthenticatorProtocol {

        @MainActor
        func authenticate(url: URL, callbackURL: URL) async throws -> URL {
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
                            continuation.resume(throwing: error)
                        case .presentationContextInvalid, .presentationContextNotProvided:
                            continuation.resume(throwing: error)
                        @unknown default:
                            continuation.resume(throwing: error)
                        }
                    } else if let error = error {
                        continuation.resume(throwing: error)
                    }
                }

                #if os(iOS) || os(macOS)
                session.presentationContextProvider = self
                #endif

                session.start()
            }
        }
    }

    public func login(
        site: String,
        appName: String,
        appId: WpUuid?
    ) async throws -> WpApiApplicationPasswordDetails {
        let provider = AuthenticationServicesAuthenticator()
        return try await login(
            site: site,
            appName: appName,
            appId: appId,
            authenticator: provider
        )
    }
}
#endif

#if os(iOS) || os(macOS)
extension WordPressLoginClient.AuthenticationServicesAuthenticator: ASWebAuthenticationPresentationContextProviding {
    func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor {
        ASPresentationAnchor()
    }
}
#endif
