import Foundation
import WordPressAPIInternal

extension ParsedUrl {

    public func asURL() -> URL {
        // `rfc3986Url()` percent-encodes the characters Rust's `url` leaves as they are, such as `|` and `{`.
        // Before iOS 17 and macOS 14, `URL(string:)` returns `nil` for them instead of encoding them itself.
        guard let result = URL(string: rfc3986Url()) else {
            fatalError("`ParsedUrl` is not a `URL`: \(url())")
        }
        return result
    }

}
