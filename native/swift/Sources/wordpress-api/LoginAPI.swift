import Foundation

#if canImport(WordPressAPIInternal)
import WordPressAPIInternal
#endif

#if os(Linux)
import FoundationNetworking
#endif

public class WordPressLoginClient {

    private let urlSession: URLSession

    public init(urlSession: URLSession) {
        self.urlSession = urlSession
    }

    public func discoverLoginUrl(for proposedSiteUrl: String) async throws -> UrlDiscoverySuccess {
        let client = UniffiWpLoginClient(requestExecutor: self.urlSession)

        do {
            return try await client.apiDiscovery(siteUrl: proposedSiteUrl)
        } catch let err {
            guard let discoveryError = err as? UrlDiscoveryError else {
                throw err
            }

            // TODO: Once we adopt Swift 6 this can be a typed throw
            throw discoveryError
        }

    }
}
