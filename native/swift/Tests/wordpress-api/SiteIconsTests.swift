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
        #expect(icons.map(\.url) == ["https://example.com/icon-32.png", "https://example.com/blog/touch.png"])
        #expect(icons.map(\.kind) == [.icon, .appleTouchIcon])
        #expect(icons.first?.sizes == [.pixels(width: 32, height: 32)])
    }

    @Test
    func picksTheIconForASize() throws {
        let pageURL = try #require(URL(string: "https://example.com/blog/"))
        let icon = try #require(try SiteIcon.preferred(inHTML: Self.html, pageURL: pageURL, size: 120))
        #expect(icon.asURL == URL(string: "https://example.com/blog/touch.png"))
    }

    @Test
    func fallsBackToFaviconIco() throws {
        let pageURL = try #require(URL(string: "https://example.com/blog/"))
        let icon = try #require(try SiteIcon.preferred(inHTML: "<p>No icons</p>", pageURL: pageURL, size: 120))
        #expect(icon.kind == .defaultFavicon)
        #expect(icon.url == "https://example.com/favicon.ico")
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
        #expect(icon.url == "https://example.com/icon.png?v=%7Bhash%7D%7C2%5Ex%5B0%5D")
        #expect(icon.asURL.absoluteString == icon.url)
        if #available(macOS 14, iOS 17, tvOS 17, watchOS 10, *) {
            #expect(URL(string: icon.url, encodingInvalidCharacters: false) != nil)
        }
    }

    @Test
    func iconsCanBeComparedAndHashed() throws {
        let pageURL = try #require(URL(string: "https://example.com/blog/"))
        let first = try SiteIcon.all(inHTML: Self.html, pageURL: pageURL)
        let second = try SiteIcon.all(inHTML: Self.html, pageURL: pageURL)
        #expect(first == second)
        #expect(Set(first + second).count == 2)
    }
}
