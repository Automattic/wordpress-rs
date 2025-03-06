package rs.wordpress.api.kotlin

import kotlin.test.assertEquals
import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.ParseApiRootUrlException
import uniffi.wp_api.WpNetworkHeaderMap
import uniffi.wp_api.localizeParseApiRootUrlError
import uniffi.wp_localization.WpLocale

class LocalizationTest {
    @Test
    fun testLocalizeParseApiRootUrlError() = runTest {
        val error = ParseApiRootUrlException.ApiRootLinkHeaderNotFound(
            statusCode = 404u,
            headerMap = WpNetworkHeaderMap.fromMap(mapOf())
        )
        assertEquals(
            "WordPress REST API link is not found in the site response",
            localizeParseApiRootUrlError(error, locale = null)
        )
        assertEquals(
            "WordPress REST API bağlantısı site yanıtında bulunamadı",
            localizeParseApiRootUrlError(error, locale = "tr-TR")
        )
    }
}
