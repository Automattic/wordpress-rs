package rs.wordpress.api.kotlin

import org.junit.jupiter.api.Test
import org.junit.jupiter.params.ParameterizedTest
import org.junit.jupiter.params.provider.Arguments
import org.junit.jupiter.params.provider.MethodSource
import uniffi.wp_api.WpComLanguage
import java.util.Locale
import java.util.stream.Stream
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class WpComLanguageTest {

    @ParameterizedTest(name = "Language code {0} should map to {1}")
    @MethodSource("simpleLanguageCodeProvider")
    fun testSimpleLanguageCodeConversion(localeIdentifier: String, expected: WpComLanguage) {
        val locale = Locale.forLanguageTag(localeIdentifier)
        val result = WpComLanguage.fromLocale(locale)
        assertNotNull(result, "Expected $expected for locale $localeIdentifier but got null")
        assertEquals(expected, result, "Locale $localeIdentifier should map to $expected")
    }

    @ParameterizedTest(name = "Regional locale {0} should map to {1}")
    @MethodSource("regionalLocaleProvider")
    fun testRegionalLocaleConversion(localeIdentifier: String, expected: WpComLanguage) {
        val locale = Locale.forLanguageTag(localeIdentifier)
        val result = WpComLanguage.fromLocale(locale)
        assertNotNull(result, "Expected $expected for locale $localeIdentifier but got null")
        assertEquals(expected, result, "Locale $localeIdentifier should map to $expected")
    }

    @Test
    fun testChineseSimplifiedScript() {
        val locale = Locale.forLanguageTag("zh-Hans")
        val result = WpComLanguage.fromLocale(locale)
        assertEquals(WpComLanguage.CHINESE_SIMPLIFIED, result)
    }

    @Test
    fun testChineseTraditionalScript() {
        val locale = Locale.forLanguageTag("zh-Hant")
        val result = WpComLanguage.fromLocale(locale)
        assertEquals(WpComLanguage.CHINESE_TRADITIONAL, result)
    }

    @Test
    fun testChineseHongKong() {
        val locale = Locale.forLanguageTag("zh-Hant-HK")
        val result = WpComLanguage.fromLocale(locale)
        assertEquals(WpComLanguage.CHINESE_HONG_KONG, result)
    }

    @Test
    fun testChineseSingapore() {
        val locale = Locale.forLanguageTag("zh-Hans-SG")
        val result = WpComLanguage.fromLocale(locale)
        assertEquals(WpComLanguage.CHINESE_SINGAPORE, result)
    }

    @Test
    fun testUnknownLocaleReturnsNull() {
        val locale = Locale.forLanguageTag("xyz")
        val result = WpComLanguage.fromLocale(locale)
        assertNull(result, "Unknown locale should return null")
    }

    @Test
    fun testFromLanguageCodeSpecialCases() {
        // Belarusian: ISO 639-1 is "be", WPCom uses "bel"
        assertEquals(WpComLanguage.BELARUSIAN, WpComLanguage.fromLanguageCode("be"))

        // Sindhi: ISO 639-1 is "sd", WPCom uses "snd"
        assertEquals(WpComLanguage.SINDHI, WpComLanguage.fromLanguageCode("sd"))

        // Kyrgyz: ISO 639-1 is "ky", WPCom uses "kir"
        assertEquals(WpComLanguage.KYRGYZ, WpComLanguage.fromLanguageCode("ky"))
    }

    @Test
    fun testAllLanguages() {
        val all = WpComLanguage.all
        assertNotNull(all)
        assertEquals(149, all.size, "Expected 149 languages")
    }

    @Test
    fun testPopularLanguages() {
        val popular = WpComLanguage.popular
        assertNotNull(popular)
        assertEquals(17, popular.size, "Expected 17 popular languages")
        assertEquals(WpComLanguage.ENGLISH, popular.first(), "English should be the most popular")
    }

    companion object {
        @JvmStatic
        fun simpleLanguageCodeProvider(): Stream<Arguments> = Stream.of(
            // A
            Arguments.of("af", WpComLanguage.AFRIKAANS),
            Arguments.of("am", WpComLanguage.AMHARIC),
            Arguments.of("ar", WpComLanguage.ARABIC),
            Arguments.of("as", WpComLanguage.ASSAMESE),
            Arguments.of("ast", WpComLanguage.ASTURIAN),
            Arguments.of("az", WpComLanguage.AZERBAIJANI),
            // B
            Arguments.of("ba", WpComLanguage.BASHKIR),
            Arguments.of("bg", WpComLanguage.BULGARIAN),
            Arguments.of("bm", WpComLanguage.BAMBARA),
            Arguments.of("bn", WpComLanguage.BENGALI),
            Arguments.of("bo", WpComLanguage.TIBETAN),
            Arguments.of("br", WpComLanguage.BRETON),
            Arguments.of("bs", WpComLanguage.BOSNIAN),
            // C
            Arguments.of("ca", WpComLanguage.CATALAN),
            Arguments.of("ce", WpComLanguage.CHECHEN),
            Arguments.of("ckb", WpComLanguage.CENTRAL_KURDISH),
            Arguments.of("cs", WpComLanguage.CZECH),
            Arguments.of("cv", WpComLanguage.CHUVASH),
            Arguments.of("cy", WpComLanguage.WELSH),
            // D
            Arguments.of("da", WpComLanguage.DANISH),
            Arguments.of("de", WpComLanguage.GERMAN),
            Arguments.of("dv", WpComLanguage.DHIVEHI),
            Arguments.of("dz", WpComLanguage.DZONGKHA),
            // E
            Arguments.of("el", WpComLanguage.GREEK),
            Arguments.of("en", WpComLanguage.ENGLISH),
            Arguments.of("eo", WpComLanguage.ESPERANTO),
            Arguments.of("es", WpComLanguage.SPANISH),
            Arguments.of("et", WpComLanguage.ESTONIAN),
            Arguments.of("eu", WpComLanguage.BASQUE),
            // F
            Arguments.of("fa", WpComLanguage.PERSIAN),
            Arguments.of("fi", WpComLanguage.FINNISH),
            Arguments.of("fo", WpComLanguage.FAROESE),
            Arguments.of("fr", WpComLanguage.FRENCH),
            Arguments.of("fur", WpComLanguage.FRIULIAN),
            Arguments.of("fy", WpComLanguage.WESTERN_FRISIAN),
            // G
            Arguments.of("ga", WpComLanguage.IRISH),
            Arguments.of("gd", WpComLanguage.SCOTTISH_GAELIC),
            Arguments.of("gl", WpComLanguage.GALICIAN),
            Arguments.of("gn", WpComLanguage.GUARANI),
            Arguments.of("gu", WpComLanguage.GUJARATI),
            // H
            Arguments.of("he", WpComLanguage.HEBREW),
            Arguments.of("hi", WpComLanguage.HINDI),
            Arguments.of("hr", WpComLanguage.CROATIAN),
            Arguments.of("hu", WpComLanguage.HUNGARIAN),
            Arguments.of("hy", WpComLanguage.ARMENIAN),
            // I
            Arguments.of("ia", WpComLanguage.INTERLINGUA),
            Arguments.of("id", WpComLanguage.INDONESIAN),
            Arguments.of("ii", WpComLanguage.NUOSU_YI),
            Arguments.of("is", WpComLanguage.ICELANDIC),
            Arguments.of("it", WpComLanguage.ITALIAN),
            // J
            Arguments.of("ja", WpComLanguage.JAPANESE),
            // K
            Arguments.of("ka", WpComLanguage.GEORGIAN),
            Arguments.of("kab", WpComLanguage.KABYLE),
            Arguments.of("kk", WpComLanguage.KAZAKH),
            Arguments.of("km", WpComLanguage.KHMER),
            Arguments.of("kn", WpComLanguage.KANNADA),
            Arguments.of("ko", WpComLanguage.KOREAN),
            Arguments.of("ks", WpComLanguage.KASHMIRI),
            // L
            Arguments.of("lo", WpComLanguage.LAO),
            Arguments.of("lt", WpComLanguage.LITHUANIAN),
            Arguments.of("lv", WpComLanguage.LATVIAN),
            // M
            Arguments.of("mk", WpComLanguage.MACEDONIAN),
            Arguments.of("ml", WpComLanguage.MALAYALAM),
            Arguments.of("mn", WpComLanguage.MONGOLIAN),
            Arguments.of("mr", WpComLanguage.MARATHI),
            Arguments.of("ms", WpComLanguage.MALAY),
            Arguments.of("mt", WpComLanguage.MALTESE),
            // N
            Arguments.of("nb", WpComLanguage.NORWEGIAN_BOKMAL),
            Arguments.of("nds", WpComLanguage.LOW_GERMAN),
            Arguments.of("ne", WpComLanguage.NEPALI),
            Arguments.of("nl", WpComLanguage.DUTCH),
            Arguments.of("nn", WpComLanguage.NORWEGIAN_NYNORSK),
            Arguments.of("nv", WpComLanguage.NAVAJO),
            // O
            Arguments.of("or", WpComLanguage.ODIA),
            Arguments.of("os", WpComLanguage.OSSETIC),
            // P
            Arguments.of("pa", WpComLanguage.PUNJABI),
            Arguments.of("pl", WpComLanguage.POLISH),
            Arguments.of("ps", WpComLanguage.PASHTO),
            Arguments.of("pt", WpComLanguage.PORTUGUESE),
            // Q
            Arguments.of("qu", WpComLanguage.QUECHUA),
            // R
            Arguments.of("ro", WpComLanguage.ROMANIAN),
            Arguments.of("ru", WpComLanguage.RUSSIAN),
            // S
            Arguments.of("sc", WpComLanguage.SARDINIAN),
            Arguments.of("si", WpComLanguage.SINHALA),
            Arguments.of("sk", WpComLanguage.SLOVAK),
            Arguments.of("sl", WpComLanguage.SLOVENIAN),
            Arguments.of("so", WpComLanguage.SOMALI),
            Arguments.of("sq", WpComLanguage.ALBANIAN),
            Arguments.of("sr", WpComLanguage.SERBIAN),
            Arguments.of("su", WpComLanguage.SUNDANESE),
            Arguments.of("sv", WpComLanguage.SWEDISH),
            // T
            Arguments.of("ta", WpComLanguage.TAMIL),
            Arguments.of("te", WpComLanguage.TELUGU),
            Arguments.of("th", WpComLanguage.THAI),
            Arguments.of("tr", WpComLanguage.TURKISH),
            Arguments.of("tt", WpComLanguage.TATAR),
            // U
            Arguments.of("ug", WpComLanguage.UYGHUR),
            Arguments.of("uk", WpComLanguage.UKRAINIAN),
            Arguments.of("ur", WpComLanguage.URDU),
            Arguments.of("uz", WpComLanguage.UZBEK),
            // V
            Arguments.of("vec", WpComLanguage.VENETIAN),
            Arguments.of("vi", WpComLanguage.VIETNAMESE),
            // W
            Arguments.of("wa", WpComLanguage.WALLOON),
            // Y
            Arguments.of("yi", WpComLanguage.YIDDISH),
            Arguments.of("yo", WpComLanguage.YORUBA),
            // Z
            Arguments.of("za", WpComLanguage.ZHUANG)
        )

        @JvmStatic
        fun regionalLocaleProvider(): Stream<Arguments> = Stream.of(
            // English variants
            Arguments.of("en-GB", WpComLanguage.ENGLISH_UK),
            Arguments.of("en-US", WpComLanguage.ENGLISH),
            Arguments.of("en-AU", WpComLanguage.ENGLISH),
            // French variants
            Arguments.of("fr-CA", WpComLanguage.FRENCH_CANADA),
            Arguments.of("fr-CH", WpComLanguage.FRENCH_SWITZERLAND),
            Arguments.of("fr-BE", WpComLanguage.FRENCH_BELGIUM),
            Arguments.of("fr-FR", WpComLanguage.FRENCH),
            // German variants
            Arguments.of("de-AT", WpComLanguage.GERMAN_AUSTRIA),
            Arguments.of("de-CH", WpComLanguage.GERMAN_SWITZERLAND),
            Arguments.of("de-DE", WpComLanguage.GERMAN),
            // Spanish variants
            Arguments.of("es-MX", WpComLanguage.SPANISH_MEXICO),
            Arguments.of("es-CL", WpComLanguage.SPANISH_CHILE),
            Arguments.of("es-ES", WpComLanguage.SPANISH),
            // Portuguese variants
            Arguments.of("pt-BR", WpComLanguage.PORTUGUESE_BRAZIL),
            Arguments.of("pt-PT", WpComLanguage.PORTUGUESE),
            // Dutch variants
            Arguments.of("nl-BE", WpComLanguage.DUTCH_BELGIUM),
            Arguments.of("nl-NL", WpComLanguage.DUTCH)
        )
    }
}
