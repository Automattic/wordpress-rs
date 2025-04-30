import Foundation
import WordPressAPIInternal

#if os(Linux)
import FoundationNetworking
#endif

extension JetpackConnectionClient {
    public convenience init(
        apiRootUrl: ParsedUrl,
        urlSession: URLSession,
        authentication: WpAuthentication,
        middlewarePipeline: MiddlewarePipeline = .default
    ) {
        self.init(
            apiRootUrl: apiRootUrl,
            delegate: .init(
                authProvider: .staticWithAuth(auth: authentication),
                requestExecutor: WpRequestExecutor(urlSession: urlSession),
                middlewarePipeline: middlewarePipeline,
                appNotifier: AppNotifier()
            )
        )
    }
}
