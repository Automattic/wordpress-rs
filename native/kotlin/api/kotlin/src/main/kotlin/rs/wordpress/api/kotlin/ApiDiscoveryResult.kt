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

/**
 * A concise description of a failed discovery attempt for diagnostics, or
 * `null` when discovery succeeded.
 *
 * [policy] decides how much of the URLs involved and of the API root's response
 * the description carries, exactly as it does for [WpRequestResult].
 *
 * Intended for logs and crash reporting ONLY. Never surface this to users;
 * [userFacingErrorMessage] is the one to show.
 */
fun ApiDiscoveryResult.toLogErrorString(
    policy: RequestErrorLogPolicy = RequestErrorLogPolicy.DEFAULT
): String? = when (this) {
    is ApiDiscoveryResult.Success -> null
    // The site URL is what failed to parse, so there is no URL to report, and
    // the parse error names no part of the input.
    is ApiDiscoveryResult.FailureParseSiteUrl ->
        "FailureParseSiteUrl(reason=${error::class.simpleName})"
    is ApiDiscoveryResult.FailureFindApiRoot ->
        "FailureFindApiRoot(siteUrl=${policy.redactedUrl(parsedSiteUrl.toString())}, " +
            "reason=${findApiRootFailure.toLogString(policy)})"
    is ApiDiscoveryResult.FailureFetchAndParseApiRoot ->
        "FailureFetchAndParseApiRoot(siteUrl=${policy.redactedUrl(parsedSiteUrl.toString())}, " +
            "apiRootUrl=${policy.redactedUrl(apiRootUrl.toString())}, " +
            "reason=${fetchAndParseApiRootFailure.toLogString(policy)})"
}

private fun FindApiRootFailure.toLogString(policy: RequestErrorLogPolicy): String = when (this) {
    is FindApiRootFailure.FetchHomepage -> "FetchHomepage(${error.toLogString(policy)})"
    is FindApiRootFailure.ProbablyNotAWordPressSite -> "ProbablyNotAWordPressSite"
    is FindApiRootFailure.RestApiDisabled -> "RestApiDisabled"
}

private fun FetchAndParseApiRootFailure.toLogString(policy: RequestErrorLogPolicy): String =
    when (this) {
        is FetchAndParseApiRootFailure.FetchApiRoot ->
            "FetchApiRoot(${error.toLogString(policy)})"
        is FetchAndParseApiRootFailure.ParseApiRoot ->
            "ParseApiRoot(bodyType=$responseBodyType, reason=$reason" +
                policy.responseTextField("parsingError", parsingErrorMessage) +
                policy.responseField(responseBody) + ")"
        is FetchAndParseApiRootFailure.WpError ->
            "WpError(code=${errorCode::class.simpleName}, status=$statusCode" +
                policy.responseTextField("message", errorMessage) + ")"
        is FetchAndParseApiRootFailure.ApplicationPasswordsNotSupported ->
            "ApplicationPasswordsNotSupported(reason=$reason)"
    }

private fun RequestExecutionException.toLogString(policy: RequestErrorLogPolicy): String =
    when (this) {
        is RequestExecutionException.RequestExecutionFailed ->
            "RequestExecutionFailed(status=$statusCode, reason=$reason, " +
                "method=$requestMethod, url=${policy.redactedUrl(requestUrl)})"
        is RequestExecutionException.MediaFileNotFound -> "MediaFileNotFound(path=$filePath)"
        is RequestExecutionException.MediaFileUnreadable -> "MediaFileUnreadable(path=$filePath)"
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
