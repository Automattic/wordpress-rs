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
        let icon = try SiteIcon.preferred(inHTML: Self.html, pageURL: pageURL, size: 120)
        #expect(icon.asURL == URL(string: "https://example.com/blog/touch.png"))
    }

    @Test
    func fallsBackToFaviconIco() throws {
        let pageURL = try #require(URL(string: "https://example.com/blog/"))
        let icon = try SiteIcon.preferred(inHTML: "<p>No icons</p>", pageURL: pageURL, size: 120)
        #expect(icon.kind == .defaultFavicon)
        #expect(icon.url == "https://example.com/favicon.ico")
    }
}
