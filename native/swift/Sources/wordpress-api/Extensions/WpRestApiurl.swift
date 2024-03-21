import Foundation
import wordpress_api_wrapper

enum WPRestAPIError: Error {
    case invalidUrl
}

public extension WpRestApiurl {
    func asUrl() throws -> URL {
        guard
            let url = URL(string: stringValue),
            url.scheme != nil,
            url.host != nil
        else {
            throw WPRestAPIError.invalidUrl
        }

        return url
    }
}
