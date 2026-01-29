package rs.wordpress.api.kotlin

import uniffi.wp_api.WpComLanguage
import uniffi.wp_api.wpComLanguageFromSlug
import java.util.Locale

/**
 * Extension functions for converting between Java/Android [Locale] and [WpComLanguage].
 */

/**
 * Creates a [WpComLanguage] from a Java [Locale].
 *
 * This handles regional variants by trying the combined language-region slug first
 * (e.g., "pt-br", "en-gb", "fr-ca"), then falling back to the language code alone.
 *
 * For Chinese locales, it handles script variants:
 * - Simplified (Hans) -> Chinese Simplified
 * - Traditional (Hant) -> Chinese Traditional
 * - With regional overrides for Hong Kong (zh-hk) and Singapore (zh-sg)
 *
 * @return The matching [WpComLanguage], or null if no match is found.
 */
@Suppress("CyclomaticComplexMethod", "ReturnCount") // Sequential fallback logic benefits from early returns
fun WpComLanguage.Companion.fromLocale(locale: Locale): WpComLanguage? {
    val languageCode = locale.language
    val regionCode = locale.country
    val script = locale.script

    // Try combined language-region slug first (e.g., "pt-br", "en-gb", "fr-ca")
    if (languageCode.isNotEmpty() && regionCode.isNotEmpty()) {
        val combinedSlug = "$languageCode-$regionCode".lowercase()
        wpComLanguageFromSlug(combinedSlug)?.let { return it }
    }

    // Handle Chinese script variants (Hans -> zh-cn, Hant -> zh-tw, with regional overrides)
    if (languageCode == "zh") {
        // Try region-specific Chinese variant first (zh-hk, zh-sg)
        if (regionCode.isNotEmpty()) {
            val regionSlug = "zh-$regionCode".lowercase()
            wpComLanguageFromSlug(regionSlug)?.let { return it }
        }
        // Fall back to script-based variant
        return when (script) {
            "Hans" -> WpComLanguage.CHINESE_SIMPLIFIED
            "Hant" -> WpComLanguage.CHINESE_TRADITIONAL
            else -> WpComLanguage.CHINESE_SIMPLIFIED // Default to simplified
        }
    }

    // Handle special cases where WP.com uses different codes than ISO 639-1
    val specialCasesResult = when (languageCode) {
        "be" -> WpComLanguage.BELARUSIAN // ISO 639-1: be, WPCom uses: bel
        "sd" -> WpComLanguage.SINDHI // ISO 639-1: sd, WPCom uses: snd
        "ky" -> WpComLanguage.KYRGYZ // ISO 639-1: ky, WPCom uses: kir
        else -> null
    }
    if (specialCasesResult != null) return specialCasesResult

    // Try language code alone
    if (languageCode.isNotEmpty()) {
        wpComLanguageFromSlug(languageCode)?.let { return it }
    }

    // Try the full locale tag with underscore replaced by hyphen
    val localeTag = locale.toLanguageTag().replace("_", "-").lowercase()
    wpComLanguageFromSlug(localeTag)?.let { return it }

    return null
}

/**
 * Creates a [WpComLanguage] from a language code string.
 *
 * @param languageCode The ISO 639-1 or ISO 639-2 language code (e.g., "en", "pt", "zh")
 * @return The matching [WpComLanguage], or null if no match is found.
 */
fun WpComLanguage.Companion.fromLanguageCode(languageCode: String): WpComLanguage? {
    // Handle special cases where WP.com uses different codes than ISO 639-1
    return when (languageCode.lowercase()) {
        "be" -> WpComLanguage.BELARUSIAN // ISO 639-1: be, WPCom uses: bel
        "sd" -> WpComLanguage.SINDHI // ISO 639-1: sd, WPCom uses: snd
        "ky" -> WpComLanguage.KYRGYZ // ISO 639-1: ky, WPCom uses: kir
        else -> wpComLanguageFromSlug(languageCode.lowercase())
    }
}

/**
 * Returns all available [WpComLanguage] values.
 */
val WpComLanguage.Companion.all: List<WpComLanguage>
    get() = uniffi.wp_api.wpComLanguageAll()

/**
 * Returns the popular [WpComLanguage] values, sorted by popularity rank.
 */
val WpComLanguage.Companion.popular: List<WpComLanguage>
    get() = uniffi.wp_api.wpComLanguagePopular()
