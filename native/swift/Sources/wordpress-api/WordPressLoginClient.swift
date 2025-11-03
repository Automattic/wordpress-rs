import Foundation
import WordPressAPIInternal

#if os(Linux)
import FoundationNetworking
#endif

public final class WordPressLoginClient: @unchecked Sendable {

    private let requestExecutor: SafeRequestExecutor
    private let client: UniffiWpLoginClient
    private let middleware: MiddlewarePipeline

    public convenience init(
        urlSession: URLSession,
        middleware: MiddlewarePipeline = .default
    ) {
        precondition(urlSession.configuration.httpCookieStorage != nil)
        precondition(urlSession.configuration.httpShouldSetCookies == true)
        precondition(urlSession.configuration.httpCookieAcceptPolicy != .never)

        self.init(requestExecutor: WpRequestExecutor(urlSession: urlSession), middleware: middleware)
    }

    init(
        requestExecutor: SafeRequestExecutor,
        middleware: MiddlewarePipeline = .default
    ) {
        self.requestExecutor = requestExecutor
        self.middleware = middleware
        self.client = UniffiWpLoginClient(requestExecutor: requestExecutor, middlewarePipeline: middleware)
    }

    /// Uses the proposed site URL to scan the website and return login related information about the site.
    ///
    public func details(
        ofSite proposedSiteUrl: String
    ) async throws -> AutoDiscoveryAttemptSuccess {
        try await client.apiDiscovery(siteUrl: proposedSiteUrl)
    }

    /// Uses the proposed site URL to scan the website it points to and find the Application Passwords login URL
    ///
    public func findLoginUrl(
        forSite proposedSiteUrl: String
    ) async throws -> ParsedUrl {
        // All sites should have some form of authentication we can use
        try await client.apiDiscovery(siteUrl: proposedSiteUrl).applicationPasswordsAuthenticationUrl
    }

    /// Uses the proposed site URL to scan the website it points to and find the Application Passwords login URL,
    /// then creates a URL that can be displayed by `ASWebAuthenticationSession`.
    ///
    /// This method uses `findLoginUrl:` under the hood, but you should prefer this method unless you really
    /// need access to the raw login URL.
    ///
    public func loginURL(
        forSite proposedSiteUrl: String,
        application: Application
    ) async throws -> URL {
        try await client.apiDiscovery(siteUrl: proposedSiteUrl).loginURL(for: application)
    }

    /// Convert the callback URL into a set of authentication credentials
    ///
    public func credentials(from callbackUrl: URL) throws -> WpApiApplicationPasswordDetails {
        try extractLoginDetailsFromUrl(url: callbackUrl.absoluteString)
    }

    public func authenticateTemporarily(
        username: String,
        password: String,
        details: AutoDiscoveryAttemptSuccess
    ) async throws -> WordPressAPI {
        let nonceRetrieval = WpRestNonceRetrieval(details: details, requestExecutor: requestExecutor)
        let nonce = try await nonceRetrieval.getNonce(username: username, password: password)
        return WordPressAPI(
            apiUrlResolver: WpOrgSiteApiUrlResolver(apiRootUrl: details.apiRootUrl),
            authenticationProvider: .staticWithAuth(auth: .nonce(nonce: nonce)),
            executor: requestExecutor,
            middlewarePipeline: middleware,
            appNotifier: nil
        )
    }
}

extension AutoDiscoveryAttemptSuccess {

    public func loginURL(for application: Application) -> URL {
        createApplicationPasswordAuthenticationUrl(
            loginUrl: applicationPasswordsAuthenticationUrl,
            appName: application.name,
            appId: application.id,
            successUrl: application.successCallbackUrl,
            rejectUrl: application.failureCallbackUrl
        ).asURL()
    }
}

public struct Application {
    let id: WpUuid
    let name: String

    let successCallbackUrl: String
    let failureCallbackUrl: String

    public init(id: WpUuid, name: String, successCallbackUrl: String, failureCallbackUrl: String) {
        self.id = id
        self.name = name
        self.successCallbackUrl = successCallbackUrl
        self.failureCallbackUrl = failureCallbackUrl
    }

    public init(id: WpUuid, name: String, callbackUrl: String) {
        self.id = id
        self.name = name
        self.successCallbackUrl = callbackUrl
        self.failureCallbackUrl = callbackUrl
    }
}
