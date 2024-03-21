import Foundation
import wordpress_api_wrapper

public extension URL {
    func asOAuthResponseUrl() -> OAuthResponseUrl {
        OAuthResponseUrl(stringValue: self.absoluteString)
    }
}

extension OAuthResponseUrl {
    static func new(stringValue: String) -> OAuthResponseUrl {
        OAuthResponseUrl(stringValue: stringValue)
    }
}
