import Foundation
import wordpress_api_wrapper

extension WpNetworkRequest {
    init(url: URL) {
        self.init(method: .get, url: url.absoluteString, headerMap: nil)
    }
}
