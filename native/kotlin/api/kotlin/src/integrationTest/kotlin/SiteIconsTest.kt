package rs.wordpress.api.kotlin

import org.junit.jupiter.api.Test
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.SiteIconKind
import uniffi.wp_api.preferredSiteIcon
import uniffi.wp_api.siteIcons
import kotlin.test.assertEquals

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
        assertEquals("https://example.com/blog/touch.png", icon.url)
        assertEquals(SiteIconKind.APPLE_TOUCH_ICON, icon.kind)
    }

    @Test
    fun fallsBackToFaviconIco() {
        val icon = preferredSiteIcon("<p>No icons</p>", ParsedUrl.parse("https://example.com/blog/"), 120u)
        assertEquals("https://example.com/favicon.ico", icon.url)
    }
}
