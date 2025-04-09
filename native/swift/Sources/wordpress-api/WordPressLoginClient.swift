import Foundation
import WordPressAPIInternal

#if os(Linux)
import FoundationNetworking
#endif

public extension WpLoginClient {

    convenience init(urlSession: URLSession) {
        self.init(requestExecutor: WpRequestExecutor(urlSession: urlSession), middlewarePipeline: .default)
    }

    /// Convert the callback URL into a set of authentication credentials
    ///
    public func credentials(from callbackUrl: URL) throws -> WpApiApplicationPasswordDetails {
        try extractLoginDetailsFromUrl(url: callbackUrl.absoluteString)
    }

}

public extension AutoDiscoveryAttemptSuccess {

    /// Uses the proposed site URL to scan the website it points to and find the Application Passwords login URL,
    /// then creates a URL that can be displayed by `ASWebAuthenticationSession`.
    ///
    public func loginURL(for application: Application) async throws -> URL {
        guard let passwordAuthUrl = apiDetails.findApplicationPasswordsAuthenticationUrl() else {
            preconditionFailure("No Auth URL Found")
        }

        let loginUrl = try ParsedUrl.parse(input: passwordAuthUrl)
        return createApplicationPasswordAuthenticationUrl(
            loginUrl: loginUrl,
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
