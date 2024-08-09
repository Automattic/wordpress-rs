import Foundation

#if canImport(WordPressAPIInternal)
import WordPressAPIInternal
#endif

extension ParsedUrl {

    public func asURL() -> URL {
        guard let result = URL(string: url()) else {
            // It's safe to assume Rust's url is can be parsed as `URL`.
            fatalError("`ParsedUrl` is not an `URL`: \(url())")
        }
        return result
    }

}
