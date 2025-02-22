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
            "Api root link header not found!\nStatus Code: '\u2068404\u2069'\nHeader Map: '\u2068WpNetworkHeaderMap {\n    inner: {},\n}\u2069'",
            localizeParseApiRootUrlError(error, locale = null)
        )
        assertEquals(
            "Api kök bağlantı başlığı bulunamadı!\nDurum kodu: '\u2068404\u2069'\nBaşlık Haritası: '\u2068WpNetworkHeaderMap {\n    inner: {},\n}\u2069'",
            localizeParseApiRootUrlError(error, locale = WpLocale.TR_TR)
        )
    }
}
