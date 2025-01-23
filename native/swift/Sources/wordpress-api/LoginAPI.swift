import Foundation

#if canImport(WordPressAPIInternal)
import WordPressAPIInternal
#endif

#if os(Linux)
import FoundationNetworking
#endif

public actor WordPressLoginClient {

    private let requestExecutor: SafeRequestExecutor

    public enum Error: Swift.Error {
        case invalidSiteAddress
        case missingLoginUrl
        case authenticationError(OAuthResponseUrlError)
        case invalidApplicationPasswordCallback
        case cancelled
        case unknown(Swift.Error)

        /// We don't have anything useful to tell the user – this is basically "Something went wrong, please try again"
        case generic
    }

    public init(requestExecutor: SafeRequestExecutor) {
        self.requestExecutor = requestExecutor
    }

    /// Perform login autodiscovery and build a login URL
    ///
    public func authenticationUrl(
        forSite proposedSiteUrl: String,
        appName: String,
        appId: WpUuid?,
        callbackUrl: URL
    ) async throws -> ParsedUrl {
        guard let urlString = await UniffiWpLoginClient(requestExecutor: self.requestExecutor)
            .apiDiscovery(siteUrl: proposedSiteUrl)
            .successfulAttempt?
            .apiDetails()?
            .findApplicationPasswordsAuthenticationUrl()
        else {
            throw Error.invalidSiteAddress
        }

        return createApplicationPasswordAuthenticationUrl(
            loginUrl: try ParsedUrl.parse(input: urlString),
            appName: appName,
            appId: appId,
            successUrl: callbackUrl.absoluteString,
            rejectUrl: callbackUrl.absoluteString
        )
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

    /// Perform login autodiscovery and get the raw data about the process
    ///
    public func autodiscoveryResult(forSite proposedSiteUrl: String) async -> AutoDiscoveryResult {
        await UniffiWpLoginClient(requestExecutor: self.requestExecutor)
            .apiDiscovery(siteUrl: proposedSiteUrl)
    }

    /// Parse the URL we get back from the WordPress website, turning it into login details
    ///
    public func parseAuthenticationCallback(
        _ urlWithToken: URL
    ) throws(Error) -> WpApiApplicationPasswordDetails {
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

public extension AutoDiscoveryAttemptResult {

    var couldConnectToUrl: Bool {
        // no good way to find this in isolation
        true
    }

    func getConnectionErrorMessage(for locale: Locale) -> String? {
        self.errorMessage(localeId: locale.identifier)
    }

    var couldUseHttps: Bool {
        self.apiRootUrl()?.asURL().scheme == "https"
    }

    func getHttpsErrorMessage(for locale: Locale) -> String? {
        self.errorMessage(localeId: locale.identifier)
    }

    var foundApiRoot: Bool {
        self.apiRootUrl() != nil
    }

    func getApiRootErrorMessage(for locale: Locale) -> String? {
        self.errorMessage(localeId: locale.identifier)
    }

    var foundAuthenticationUrl: Bool {
        self.apiDetails()?.findApplicationPasswordsAuthenticationUrl() != nil
    }

    func getAuthenticationUrlErrorMessage(for locale: Locale) -> String? {
        self.errorMessage(localeId: locale.identifier)
    }

    var authenticationUrl: URL? {
        guard
            let string = apiDetails()?.findApplicationPasswordsAuthenticationUrl(),
            let url = URL(string: string)
        else {
            return nil
        }

        return url
    }

    var domainWithSubdomain: String? {
        guard let scheme = apiRootUrl()?.asURL().scheme, let host = apiRootUrl()?.asURL().host else {
            return nil
        }

        return scheme + "://" + host
    }
}
