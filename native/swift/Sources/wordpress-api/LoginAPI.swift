import Foundation
import WordPressAPIInternal

public class WordPressLoginClient {

    private let urlSession: URLSession

    public init(urlSession: URLSession) {
        self.urlSession = urlSession
    }

    public func discoverLoginUrl(for proposedSiteUrl: String) async throws -> UrlDiscoverySuccess {
        let client = UniffiWpLoginClient(requestExecutor: self.urlSession)
        return try await client.apiDiscovery(siteUrl: proposedSiteUrl)
    }
}
