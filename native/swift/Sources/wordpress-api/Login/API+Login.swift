import Foundation
import wordpress_api_wrapper

public extension WordPressAPI {
    static func findRestApiEndpointRoot(forSiteUrl url: URL, using session: URLSession) async throws -> URL {
        debugPrint(url)
        let response = try await session.data(from: url)
        let url = try WordPressAPI.Helpers.findRestEndpoint(data: response.0)
        return url
    }

    func getRestAPICapabilities(forApiRoot url: URL, using session: URLSession) async throws -> WpapiDetails {
        let wpResponse = try await self.perform(request: WpNetworkRequest(method: .get, url: url, headerMap: nil))
        return try wordpress_api_wrapper.parseApiDetailsResponse(response: wpResponse)
    }
}
