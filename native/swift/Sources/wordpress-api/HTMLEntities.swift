import Foundation
import WordPressAPIInternal

extension String {
    /// This text with its HTML character references decoded, the way a browser decodes text content.
    ///
    /// Use it to show a field the API returns as HTML, such as a post title, as plain text: `&#8211;` becomes `–`
    /// and `&amp;` becomes `&`. It decodes once, and leaves tags in place.
    public func decodingHTMLEntities() -> String {
        decodeHtmlEntities(text: self)
    }
}
