package rs.wordpress.api.kotlin

import org.junit.jupiter.api.Test
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.SiteIconKind
import uniffi.wp_api.preferredSiteIcon
import uniffi.wp_api.siteIcons
import kotlin.test.assertEquals
import kotlin.test.assertNull

class SiteIconsTest {
    private val html = """
        <link rel="icon" href="/icon-32.png" sizes="32x32">
        <link rel="apple-touch-icon" href="touch.png">
    """.trimIndent()

    @Test
    fun listsTheIconsAPageDeclares() {
        val icons = siteIcons(html, ParsedUrl.parse("https://example.com/blog/"))
        assertEquals(
            listOf("https://example.com/icon-32.png", "https://example.com/blog/touch.png"),
            icons.map { it.url }
        )
    }

    @Test
    fun picksTheIconForASize() {
        val icon = preferredSiteIcon(html, ParsedUrl.parse("https://example.com/blog/"), 120u)
        assertEquals("https://example.com/blog/touch.png", icon?.url)
        assertEquals(SiteIconKind.APPLE_TOUCH_ICON, icon?.kind)
    }

    @Test
    fun fallsBackToFaviconIco() {
        val icon = preferredSiteIcon("<p>No icons</p>", ParsedUrl.parse("https://example.com/blog/"), 120u)
        assertEquals("https://example.com/favicon.ico", icon?.url)
    }

    @Test
    fun hasNoFallbackForAPageThatIsNotOnTheWeb() {
        assertNull(preferredSiteIcon("", ParsedUrl.parse("about:blank"), 120u))
    }

    @Test
    fun encodesCharactersRfc3986DoesNotAllow() {
        val icon = preferredSiteIcon(
            """<link rel="icon" href="/icon.png?v={hash}|2^x">""",
            ParsedUrl.parse("https://example.com/"),
            32u
        )
        val url = icon?.url
        assertEquals("https://example.com/icon.png?v=%7Bhash%7D%7C2%5Ex", url)
        // `java.net.URI` rejects the unencoded form, like Foundation's `URL` before iOS 17.
        assertEquals(url, java.net.URI(url).toString())
    }

    @Test
    fun convertsAParsedUrlWithCharactersRfc3986DoesNotAllow() {
        val url = ParsedUrl.parse("https://example.com/icon.png?v={hash}|2^x").toURL()
        assertEquals("https://example.com/icon.png?v=%7Bhash%7D%7C2%5Ex", url.toString())
    }
}
