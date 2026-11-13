import Foundation
import Testing
import WordPressAPI

struct SiteIconsTests {

    static let html = """
        <link rel="icon" href="/icon-32.png" sizes="32x32">
        <link rel="apple-touch-icon" href="touch.png">
        """

    @Test
    func listsTheIconsAPageDeclares() throws {
        let icons = try SiteIcon.all(inHTML: Self.html, pageURL: #require(URL(string: "https://example.com/blog/")))
        #expect(icons.map { $0.url.url() } == ["https://example.com/icon-32.png", "https://example.com/blog/touch.png"])
        #expect(icons.map(\.kind) == [.icon, .appleTouchIcon])
        #expect(icons.first?.sizes == [.pixels(width: 32, height: 32)])
    }

    @Test
    func picksTheIconForASize() throws {
        let pageURL = try #require(URL(string: "https://example.com/blog/"))
        let icon = try #require(try SiteIcon.preferred(inHTML: Self.html, pageURL: pageURL, size: 120))
        #expect(icon.url.asURL() == URL(string: "https://example.com/blog/touch.png"))
    }

    @Test
    func fallsBackToFaviconIco() throws {
        let pageURL = try #require(URL(string: "https://example.com/blog/"))
        let icon = try #require(try SiteIcon.preferred(inHTML: "<p>No icons</p>", pageURL: pageURL, size: 120))
        #expect(icon.kind == .defaultFavicon)
        #expect(icon.url.url() == "https://example.com/favicon.ico")
    }

    @Test
    func hasNoFallbackForAPageThatIsNotOnTheWeb() throws {
        let pageURL = try #require(URL(string: "about:blank"))
        #expect(try SiteIcon.preferred(inHTML: "", pageURL: pageURL, size: 120) == nil)
    }

    @Test
    func iconAddressesWithCharactersRFC3986DoesNotAllowBecomeURLs() throws {
        let pageURL = try #require(URL(string: "https://example.com/"))
        let html = #"<link rel="icon" href="/icon.png?v={hash}|2^x[0]">"#
        let icon = try #require(try SiteIcon.preferred(inHTML: html, pageURL: pageURL, size: 32))
        #expect(icon.url.asURL().absoluteString == "https://example.com/icon.png?v=%7Bhash%7D%7C2%5Ex%5B0%5D")
    }
}
