package rs.wordpress.api.kotlin

import org.junit.jupiter.api.Test
import uniffi.wp_api.AutoDiscoveryAttemptFailure
import uniffi.wp_api.FetchAndParseApiRootFailure
import uniffi.wp_api.FindApiRootFailure
import uniffi.wp_api.ParseUrlException
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.WpErrorCode
import uniffi.wp_api.localizedDescription
import uniffi.wp_localization.wpLocaleResolve
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

/**
 * Exercises the localized, translated error messages exposed to Kotlin via
 * [localizedDescription], and the discovery/login wrapper that now surfaces them.
 *
 * Mirrors the Swift `LocalizationTests`. Everything here is offline and deterministic —
 * the failures are constructed in memory, so there is no live-fixture flakiness.
 */
class LocalizationTest {

    private val enUs = wpLocaleResolve(listOf("en-US"))
    private val trTr = wpLocaleResolve(listOf("tr-TR"))

    @Test
    fun parseUrlErrorRendersLocalizedMessage() {
        val error = assertFailsWith<ParseUrlException> { ParsedUrl.parse("not-url") }

        assertEquals("URL is invalid.", error.localizedDescription(enUs))
        assertEquals("Geçersiz URL.", error.localizedDescription(trTr))
    }

    /**
     * Part 2: `RestApiDisabled` used to be mislabeled "Could not connect to $url." — the
     * failure `WpLoginClient.apiDiscovery` passes through now carries the actionable,
     * translated reason.
     */
    @Test
    fun restApiDisabledRendersLocalizedReason() {
        val failure = AutoDiscoveryAttemptFailure.FindApiRoot(
            parsedSiteUrl = ParsedUrl.parse("https://example.com"),
            findApiRootFailure = FindApiRootFailure.RestApiDisabled,
        )

        assertEquals(
            "The site's REST API is disabled. Please update your site settings to enable REST API.",
            failure.localizedDescription(enUs),
        )
    }

    /**
     * Part 2 (flagship): a private site used to collapse into "Found a site at $url but
     * failed to read its API configuration." — the server's own reason now renders.
     */
    @Test
    fun privateSiteRendersServerErrorMessage() {
        val failure = AutoDiscoveryAttemptFailure.FetchAndParseApiRoot(
            parsedSiteUrl = ParsedUrl.parse("https://private.example.com"),
            apiRootUrl = ParsedUrl.parse("https://private.example.com/wp-json"),
            fetchAndParseApiRootFailure = FetchAndParseApiRootFailure.WpError(
                errorCode = WpErrorCode.Forbidden(),
                errorMessage = "This site is private.",
                statusCode = 401u,
            ),
        )

        assertEquals("This site is private.", failure.localizedDescription(enUs))
    }
}
