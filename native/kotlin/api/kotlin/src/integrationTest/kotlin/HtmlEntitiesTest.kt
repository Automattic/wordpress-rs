package rs.wordpress.api.kotlin

import org.junit.jupiter.api.Test
import uniffi.wp_api.decodeHtmlEntities
import kotlin.test.assertEquals

class HtmlEntitiesTest {
    @Test
    fun decodesAPostTitle() {
        assertEquals(
            "WooCommerce for Android 3.5 is rolled out – 3.6 in code freeze",
            decodeHtmlEntities("WooCommerce for Android 3.5 is rolled out &#8211; 3.6 in code freeze")
        )
    }

    @Test
    fun decodesOnlyOnce() {
        assertEquals("&amp;", decodeHtmlEntities("&amp;amp;"))
    }

    @Test
    fun keepsTextThatIsNotAReference() {
        assertEquals("AT&T <em>Café</em> 😀", decodeHtmlEntities("AT&T <em>Café</em> 😀"))
    }
}
