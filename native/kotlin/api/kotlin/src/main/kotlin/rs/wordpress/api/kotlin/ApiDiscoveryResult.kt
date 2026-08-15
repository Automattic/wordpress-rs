package rs.wordpress.api.kotlin

import uniffi.wp_api.AutoDiscoveryAttemptSuccess
import uniffi.wp_api.FetchAndParseApiRootFailure
import uniffi.wp_api.FindApiRootFailure
import uniffi.wp_api.ParseUrlException
import uniffi.wp_api.RequestExecutionErrorReason
import uniffi.wp_api.RequestExecutionException
import java.net.URL

sealed class ApiDiscoveryResult {
    data class Success(val success: AutoDiscoveryAttemptSuccess) : ApiDiscoveryResult()
    data class FailureParseSiteUrl(
        val error: ParseUrlException
    ) : ApiDiscoveryResult()
    data class FailureFindApiRoot(
        val parsedSiteUrl: URL,
        val findApiRootFailure: FindApiRootFailure
    ) : ApiDiscoveryResult()
    data class FailureFetchAndParseApiRoot(
        val parsedSiteUrl: URL,
        val apiRootUrl: URL,
        val fetchAndParseApiRootFailure: FetchAndParseApiRootFailure
    ) : ApiDiscoveryResult()

    /**
     * Returns a user-facing error message for failed discovery attempts, or `null` on success.
     *
     * @param url The site URL that was used for discovery, included in messages for context.
     */
    fun userFacingErrorMessage(url: String): String? = when (this) {
        is Success -> null
        is FailureParseSiteUrl -> "Invalid site URL: $url"
        is FailureFindApiRoot -> findApiRootFailure.userFacingMessage(url)
        is FailureFetchAndParseApiRoot -> "Found a site at $url but failed to read its API configuration."
    }
}

private fun FindApiRootFailure.userFacingMessage(url: String): String {
    val reason = when (this) {
        is FindApiRootFailure.FetchHomepage ->
            (error as? RequestExecutionException.RequestExecutionFailed)?.reason
        else -> null
    }
    return when (reason) {
        is RequestExecutionErrorReason.DeviceIsOfflineError ->
            "No internet connection. Please check your network settings and try again."
        is RequestExecutionErrorReason.NonExistentSiteError ->
            "Could not find a site at $url. Check the URL and try again."
        is RequestExecutionErrorReason.InvalidSslError ->
            "SSL certificate error for $url."
        is RequestExecutionErrorReason.HttpAuthenticationRequiredError ->
            "$url requires HTTP authentication."
        is RequestExecutionErrorReason.HttpAuthenticationRejectedError ->
            "HTTP authentication credentials were rejected by $url."
        is RequestExecutionErrorReason.MisconfiguredRateLimitError ->
            "$url is rate-limiting requests. Try again later."
        else -> when (this) {
            is FindApiRootFailure.ProbablyNotAWordPressSite ->
                "$url does not appear to be a WordPress site."
            else -> "Could not connect to $url."
        }
    }
}
