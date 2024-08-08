import Foundation
import AuthenticationServices

#if canImport(WordPressAPIInternal)
import WordPressAPIInternal
#endif

#if os(Linux)
import FoundationNetworking
#endif

public final class WordPressLoginClient {

    public protocol Authenticator {
        func authenticate(url: URL, callbackURL: URL) async -> Result<URL, Error>
    }

    private static let callbackURL = URL(string: "x-wordpress-app://login-callback")!

    public enum Error: Swift.Error {
        case invalidSiteAddress(UrlDiscoveryError)
        case missingLoginUrl
        case authenticationError(ApplicationPasswordAuthenticationError)
        case invalidApplicationPasswordCallback
        case cancelled
        case unknown(Swift.Error)
    }

    private let requestExecutor: SafeRequestExecutor

    public convenience init(urlSession: URLSession) {
        self.init(requestExecutor: urlSession)
    }

    init(requestExecutor: SafeRequestExecutor) {
        self.requestExecutor = requestExecutor
    }

    public func login(
        site: String,
        appName: String,
        appId: String?,
        contextProvider: ASWebAuthenticationPresentationContextProviding
    ) async -> Result<ApplicationPasswordAuthenticationSuccess, Error> {
        await login(
            site: site,
            appName: appName,
            appId: appId,
            authenticator: AuthenticationServiceAuthenticator(contextProvider: contextProvider)
        )
    }

    public func login(
        site: String,
        appName: String,
        appId: String?,
        authenticator: Authenticator
    ) async -> Result<ApplicationPasswordAuthenticationSuccess, Error> {
        let loginURL = await self.loginURL(forSite: site)
        let authURL = loginURL
            .map { loginURL in
                ApplicationPasswordAuthenticationRequest(
                    appName: appName,
                    appId: appId,
                    successUrl: Self.callbackURL.absoluteString,
                    rejectUrl: Self.callbackURL.absoluteString
                )
                .authUrl(loginUrl: loginURL)
                .asURL()
            }

        switch authURL {
        case let .failure(error):
            return .failure(error)
        case let .success(authURL):
            return await authenticator.authenticate(url: authURL, callbackURL: Self.callbackURL)
                .flatMap(handleAuthenticationCallback(_:))
        }
    }

    private func loginURL(forSite proposedSiteUrl: String) async -> Result<ParsedUrl, Error> {
        let result: UrlDiscoverySuccess
        do {
            let client = UniffiWpLoginClient(requestExecutor: self.requestExecutor)
            result = try await client.apiDiscovery(siteUrl: proposedSiteUrl)
        } catch let error as UrlDiscoveryError {
            return .failure(.invalidSiteAddress(error))
        } catch {
            return .failure(.unknown(error))
        }

        // All sites should have some form of authentication we can use
        guard let passwordAuthenticationUrl = result.apiDetails.findApplicationPasswordsAuthenticationUrl(),
              let parsedLoginUrl = try? ParsedUrl.parse(input: passwordAuthenticationUrl) else {
            return .failure(.missingLoginUrl)
        }

        return .success(parsedLoginUrl)
    }

    private func handleAuthenticationCallback(
        _ urlWithToken: URL
    ) -> Result<ApplicationPasswordAuthenticationSuccess, Error> {
        guard let parsed = try? ParsedUrl.from(url: urlWithToken) else {
            return .failure(.invalidApplicationPasswordCallback)
        }

        do {
            return try .success(ApplicationPasswordAuthenticationSuccess(callbackUrl: parsed))
        } catch let error as ApplicationPasswordAuthenticationError {
            return .failure(.authenticationError(error))
        } catch {
            return .failure(.unknown(error))
        }
    }
}

extension WordPressLoginClient {

    class AuthenticationServiceAuthenticator: Authenticator {
        let contextProvider: ASWebAuthenticationPresentationContextProviding

        init(contextProvider: ASWebAuthenticationPresentationContextProviding) {
            self.contextProvider = contextProvider
        }

        func authenticate(url: URL, callbackURL: URL) async -> Result<URL, WordPressLoginClient.Error> {
            await withCheckedContinuation { continuation in
                let session = ASWebAuthenticationSession(
                    url: url,
                    callbackURLScheme: callbackURL.scheme!
                ) { url, error in
                    if let url {
                        continuation.resume(returning: .success(url))
                    } else if let error = error as? ASWebAuthenticationSessionError {
                        switch error.code {
                        case .canceledLogin:
                            continuation.resume(returning: .failure(.cancelled))
                        case .presentationContextInvalid, .presentationContextNotProvided:
                            assertionFailure("An unexpected error received: \(error)")
                            continuation.resume(returning: .failure(.cancelled))
                        @unknown default:
                            continuation.resume(returning: .failure(.cancelled))
                        }
                    } else {
                        continuation.resume(returning: .failure(.invalidApplicationPasswordCallback))
                    }
                }
                session.presentationContextProvider = contextProvider
                session.start()
            }
        }
    }
    }
