import Foundation
import WordPressAPIInternal

#if os(Linux)
import FoundationNetworking
#endif

extension JetpackConnectionClient {
    public convenience init(apiRootUrl: ParsedUrl, urlSession: URLSession, authentication: WpAuthentication) {
        self.init(
            apiRootUrl: apiRootUrl,
            requestExecutor: WpRequestExecutor(urlSession: urlSession),
            siteAuthentication: authentication
        )
    }
}
