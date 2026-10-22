import Testing
import WordPressAPIInternal

struct ReaderTests {
    @Test
    func cleansPostHtml() {
        let content = ReaderPostContent(
            html: #"<p style="color: red">Hi</p><script src="//platform.x.com/widgets.js"></script>"#
        )
        #expect(content.html() == "<p>Hi</p>")
        #expect(content.scripts() == ["https://platform.x.com/widgets.js"])
    }

    @Test
    func findsImages() throws {
        let content = ReaderPostContent(
            html: #"<p><img src="a.jpg" srcset="a-300.jpg 300w, a-1024.jpg 1024w" alt="A"></p>"#
        )
        let image = try #require(content.images().first)
        #expect(image.alt == "A")
        #expect(image.urlForSize(maxDimension: 500) == "a-1024.jpg")
        #expect(content.offlineHtml() == #"<p><img alt="A" src="a.jpg"></p>"#)
        #expect(content.offlineImageUrls() == ["a.jpg"])
    }

    @Test
    func resolvesUrlsAgainstThePost() throws {
        let content = ReaderPostContent(html: #"<img src="/a.jpg">"#, url: "https://example.com/2026/10/post/")
        let image = try #require(content.images().first)
        #expect(image.url == "https://example.com/a.jpg")
    }

    @Test
    func addsPhotonSizes() throws {
        let content = ReaderPostContent(html: #"<img src="https://example.com/a.jpg" width="1000" height="500">"#)
        let image = try #require(content.withPhotonSizes().images().first)
        #expect(image.sizes.map(\.width) == [400, 800, 1000])
    }

    @Test
    func findsAnchorsAndText() throws {
        let content = ReaderPostContent(html: #"<h2 id="intro">Intro</h2><p>Body <em>text</em>.</p>"#)
        let anchor = try #require(content.anchors().first)
        #expect(anchor.id == "intro")
        #expect(anchor.kind == .heading(level: 2))
        #expect(anchor.text == "Intro")
        #expect(content.plainText() == "Intro\nBody text.")
        #expect(content.simplifiedHtml() == "<h2>Intro</h2>\n<p>Body text.</p>")
        #expect(content.isManualExcerpt(excerpt: "A summary the author wrote."))
        #expect(!content.isManualExcerpt(excerpt: "Intro Body text. [&hellip;]"))
    }

    @Test
    func fixedThemeCss() {
        let css = readerPageColorsCss(colors: .fixed(theme: .evening))
        #expect(css.contains("color-scheme: dark;"))
        #expect(css.contains("--reader-background: #3a3a3c;"))
    }

    @Test
    func systemThemeCss() {
        let white = ReaderColor(red: 255, green: 255, blue: 255, alpha: 255)
        let black = ReaderColor(red: 0, green: 0, blue: 0, alpha: 255)
        let light = ReaderThemeColors(
            text: black,
            secondaryText: black,
            tertiaryText: black,
            background: white,
            secondaryBackground: white,
            border: black,
            link: black
        )
        let dark = readerFixedThemeColors(theme: .oled)
        let css = readerPageColorsCss(colors: .system(light: light, dark: dark))
        #expect(css.contains("--reader-background: #ffffff;"))
        #expect(css.contains("@media (prefers-color-scheme: dark)"))
    }

    @Test
    func assemblesThePage() {
        let content = ReaderPostContent(html: #"<p>Hi</p><div class="fb-post"></div>"#)
        let page = content.pageHtml(options: ReaderPageOptions(colors: .fixed(theme: .sepia), language: "fa"))
        #expect(page.hasPrefix("<!DOCTYPE html>\n<html lang=\"fa\" dir=\"rtl\">"))
        #expect(page.contains(readerScript()))
        #expect(page.contains("<script async src=\"https://connect.facebook.net/"))

        let offline = content.pageHtml(options: ReaderPageOptions(colors: .fixed(theme: .sepia), offline: true))
        #expect(!offline.contains("<script async"))
    }

    @Test
    func setsTheReadersFontAndTextSize() {
        let content = ReaderPostContent(html: "<p>Hi</p>")
        let options = ReaderPageOptions(colors: .fixed(theme: .sepia), font: .serif, textScale: 1.25)
        let page = content.pageHtml(options: options)
        #expect(page.contains(readerPageTypographyCss(font: .serif, textScale: 1.25, colors: .fixed(theme: .sepia))))
        #expect(readerPageBaseUrl() == "https://wordpress.com/reader")
    }

    @Test
    func themesSayWhetherTheyFollowTheSystem() {
        #expect(ReaderPageColors.fixed(theme: .evening).appearance() == .alwaysDark)
        #expect(ReaderPageColors.fixed(theme: .sepia).appearance() == .alwaysLight)
        let oled = readerFixedThemeColors(theme: .oled)
        #expect(ReaderPageColors.system(light: oled, dark: oled).appearance() == .followsSystem)
    }

    @Test
    func classifiesLinks() {
        let post = "https://example.com/2026/10/21/post/"
        #expect(readerLink(url: "https://example.com/2026/10/21/post/#fn-1", postUrl: post) == .anchor(id: "fn-1"))
        #expect(readerLink(url: "https://wordpress.com/tag/photos") == .tag(slug: "photos"))
        #expect(readerLink(url: "mailto:someone@example.com") == .email(address: "someone@example.com"))
    }

    @Test
    func findsWhereAnImageIsInAGallery() throws {
        let content = ReaderPostContent(
            html: """
                <figure class="wp-block-gallery">
                <figure class="wp-block-image"><img src="https://example.com/a.jpg"></figure>
                <figure class="wp-block-image"><img src="https://example.com/b.jpg"></figure>
                </figure>
                """
        )
        let position = try #require(content.galleryPosition(url: "https://example.com/b-1024x768.jpg"))
        #expect(position.index == 1)
        #expect(position.gallery.images.count == 2)
    }

    @Test
    func knowsReadingTimeCutOffPostsAndWhatToPreload() {
        let content = ReaderPostContent(
            html: #"<p>The start … <a class="more-link" href="https://example.com/post/">Continue reading</a></p>"#
                + #"<img src="https://example.com/a.jpg">"#
        )
        #expect(content.readingTimeMinutes() == 1)
        #expect(content.isCutOff())
        #expect(content.preloadUrls(width: 390, scale: 3) == ["https://example.com/a.jpg"])
    }

    @Test
    func stylesheetUsesTheThemeColors() {
        #expect(readerStylesheet().contains("color: var(--reader-text);"))
    }

    @Test
    func scriptTalksToTheReaderPostHandler() {
        #expect(readerScript().contains("messageHandlers?.readerPost"))
    }
}
