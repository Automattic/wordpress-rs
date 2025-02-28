import Foundation
import WordPressAPIInternal

#if os(Linux)
import FoundationNetworking
#endif

extension WordPressOrgApiClient {

    public convenience init(urlSession: URLSession) {
        self.init(
            requestExecutor: WpRequestExecutor(urlSession: urlSession),
            middlewarePipeline: WpApiMiddlewarePipeline.default
        )
    }
}
