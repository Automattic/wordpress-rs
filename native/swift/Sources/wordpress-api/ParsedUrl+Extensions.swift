import Foundation

#if canImport(WordPressAPIInternal)
import WordPressAPIInternal
#endif

extension ParsedUrl {

    public func asURL() -> URL {
        guard let result = URL(string: url()) else {
            // It's safe to assume Rust's url can be parsed as `URL`.
            fatalError("`ParsedUrl` is not a `URL`: \(url())")
        }
        return result
    }

}
