package rs.wordpress.api.kotlin

import uniffi.wp_api.AutoDiscoveryAttemptFailure
import uniffi.wp_api.AutoDiscoveryAttemptSuccess
import uniffi.wp_api.FetchAndParseApiRootFailure
import uniffi.wp_api.FindApiRootFailure
import uniffi.wp_api.ParseUrlException
import uniffi.wp_api.localizedDescription
import java.net.URL

sealed class ApiDiscoveryResult {
    data class Success(val success: AutoDiscoveryAttemptSuccess) : ApiDiscoveryResult()

    /**
     * The site URL could not be parsed. [failure] carries the localized, translated
     * reason; read it with `localizedDescription()`.
     */
    data class FailureParseSiteUrl(
        val failure: AutoDiscoveryAttemptFailure.ParseSiteUrl
    ) : ApiDiscoveryResult() {
        val error: ParseUrlException get() = failure.error
    }

    /**
     * The API root could not be found. [failure] carries the localized, translated
     * reason; read it with `localizedDescription()`.
     */
    data class FailureFindApiRoot(
        val failure: AutoDiscoveryAttemptFailure.FindApiRoot
    ) : ApiDiscoveryResult() {
        val parsedSiteUrl: URL get() = failure.parsedSiteUrl.toURL()
        val findApiRootFailure: FindApiRootFailure get() = failure.findApiRootFailure
    }

    /**
     * A site was found but its API configuration could not be read. [failure] carries
     * the localized, translated reason — the server error, the plugin blocking
     * Application Passwords, and so on — which `localizedDescription()` surfaces without
     * collapsing the distinct cases into one message.
     */
    data class FailureFetchAndParseApiRoot(
        val failure: AutoDiscoveryAttemptFailure.FetchAndParseApiRoot
    ) : ApiDiscoveryResult() {
        val parsedSiteUrl: URL get() = failure.parsedSiteUrl.toURL()
        val apiRootUrl: URL get() = failure.apiRootUrl.toURL()
        val fetchAndParseApiRootFailure: FetchAndParseApiRootFailure
            get() = failure.fetchAndParseApiRootFailure
    }

    /**
     * The retained discovery failure, or `null` on success. Every failure variant
     * wraps an [AutoDiscoveryAttemptFailure], so callers can obtain its localized
     * message directly via `localizedDescription()`.
     */
    val failureOrNull: AutoDiscoveryAttemptFailure?
        get() = when (this) {
            is Success -> null
            is FailureParseSiteUrl -> failure
            is FailureFindApiRoot -> failure
            is FailureFetchAndParseApiRoot -> failure
        }

    /**
     * Returns the localized, translated user-facing error message for a failed
     * discovery attempt, or `null` on success.
     *
     * The message is rendered from the library's translations (the same ones iOS
     * shows), resolved to the device locale. It distinguishes every failure — a
     * private site, a plugin blocking Application Passwords, a disabled REST API,
     * a network error — rather than collapsing them into a single English string.
     */
    fun userFacingErrorMessage(): String? = failureOrNull?.localizedDescription()
}
