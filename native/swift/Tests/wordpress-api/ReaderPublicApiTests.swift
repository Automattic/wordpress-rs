import Testing
import WordPressAPI

/// The Reader's API as an app sees it, through `WordPressAPI` alone. Each type is named and each
/// function called here, so one that isn't exported fails to compile.
struct ReaderPublicApiTests {

    @Test
    func buildsAPostPage() {
        let theme: ReaderTheme = readerThemeFromId(id: "sepia") ?? .system
        let appColors: ReaderAppColors = readerDefaultAppColors(brand: .jetpack)
        let content: ReaderPostContent = ReaderPostContent(html: "<p>Hi</p>")
        let options = ReaderPageOptions(theme: theme, appColors: appColors, font: .serif)
        let page = content.pageHtml(options: options)
        #expect(page.contains(readerStylesheet()))
        #expect(page.contains(readerScript()))
        #expect(page.contains(readerPageColorsCss(theme: theme, appColors: appColors)))
        #expect(page.contains(readerPageTypographyCss(font: .serif, textScale: 1, theme: theme)))
        #expect(readerPageThemeCss(theme: .reader).contains("720px"))
        #expect(readerPageBaseUrl() == "https://wordpress.com/reader")
    }

    @Test
    func listsThemesAndBrands() {
        #expect(readerThemes().count == 8)
        #expect(readerBrands() == [.wordpress, .jetpack])
        #expect(readerBrandFromId(id: "jetpack") == .jetpack)
        let appearance: ReaderThemeAppearance = ReaderTheme.evening.appearance()
        #expect(appearance == .alwaysDark)
        let appColors = readerDefaultAppColors(brand: .wordpress)
        let colors: ReaderThemeColors = readerThemeColors(theme: .system, appColors: appColors, dark: false)
        #expect(colors.link.hex() == "#0675c4")
        let font: ReaderFont = .mono
        #expect(font != .sans)
    }

    @Test
    func convertsColors() throws {
        let color: ReaderColor = try #require(readerColorFromHex(hex: "#3858e980"))
        #expect(readerColorFromArgb(argb: color.argb()) == color)
        #expect(readerColorFromComponents(red: 1, green: 1, blue: 1, alpha: 1).hex() == "#ffffff")
    }

    @Test
    func findsWhatsInAPost() throws {
        let html = """
            <h2 id="intro">Intro</h2>
            <figure class="wp-block-gallery">
            <figure class="wp-block-image"><img src="https://example.com/a.jpg" width="1000" height="500"></figure>
            <figure class="wp-block-image"><img src="https://example.com/b.jpg"></figure>
            </figure>
            """
        let content: any ReaderPostContentProtocol = ReaderPostContent(html: html)

        let images: [ReaderPostImage] = content.withPhotonSizes().images()
        let sizes: [ReaderImageSize] = try #require(images.first).sizes
        #expect(!sizes.isEmpty)

        let galleries: [ReaderPostGallery] = content.galleries()
        #expect(galleries.first?.images.count == 2)
        let position: ReaderPostGalleryPosition? = content.galleryPosition(url: "https://example.com/b.jpg")
        #expect(position?.index == 1)

        let anchor: ReaderPostAnchor = try #require(content.anchors().first)
        let kind: ReaderPostAnchorKind = anchor.kind
        #expect(kind == .heading(level: 2))

        let mentions: [ReaderPostMention] = content.mentions()
        #expect(mentions.isEmpty)

        let link: ReaderLink = readerLink(url: "https://example.com/post/#intro", postUrl: "https://example.com/post/")
        #expect(link == .anchor(id: "intro"))
    }

}
