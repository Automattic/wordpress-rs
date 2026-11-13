import Foundation
import WordPressAPIInternal

extension SiteIcon {
    /// Every icon `html` declares, in document order: `<link rel="icon">`, `apple-touch-icon`, `mask-icon` and
    /// `msapplication-TileImage`.
    ///
    /// - Parameters:
    ///   - html: The page's HTML.
    ///   - pageURL: The address the page was downloaded from, after redirects. Relative icon addresses resolve
    ///     against it, or against the page's `<base href>`.
    public static func all(inHTML html: String, pageURL: URL) throws -> [SiteIcon] {
        siteIcons(html: html, pageUrl: try ParsedUrl.from(url: pageURL))
    }

    /// The icon in `html` that best fits a square of `size` pixels, or `/favicon.ico` at the root of `pageURL`'s
    /// origin when `html` declares no icon an app can show.
    ///
    /// It picks the smallest icon of at least `size` pixels, or the largest when all are smaller. SVG and mask
    /// icons are never picked. Returns `nil` when `html` declares no icon an app can show and `pageURL` isn't
    /// `http` or `https`, such as `about:blank`.
    public static func preferred(inHTML html: String, pageURL: URL, size: UInt32) throws -> SiteIcon? {
        preferredSiteIcon(html: html, pageUrl: try ParsedUrl.from(url: pageURL), size: size)
    }

    /// The icon's address.
    public var asURL: URL {
        // `url` only uses the characters RFC 3986 allows, which `URL` accepts on every supported OS version.
        guard let result = URL(string: url) else {
            fatalError("`SiteIcon.url` is not a `URL`: \(url)")
        }
        return result
    }
}
