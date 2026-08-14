package rs.wordpress.api.kotlin

import uniffi.wp_api.RequestExecutionErrorReason
import uniffi.wp_api.RequestMethod
import uniffi.wp_api.WpErrorCode
import uniffi.wp_api.WpRedirect
import uniffi.wp_api.redactRequestUrlForLog
import uniffi.wp_api.summarizeResponseBodyForLog

sealed class WpRequestResult<T> {
    data class Success<T>(val response: T) : WpRequestResult<T>()
    data class WpError<T>(
        val errorCode: WpErrorCode,
        val errorMessage: String,
        val statusCode: UInt,
        val response: String,
        val requestUrl: String,
        val requestMethod: RequestMethod,
    ) : WpRequestResult<T>()

    data class InvalidHttpStatusCode<T>(
        val statusCode: UInt,
        val requestUrl: String,
        val requestMethod: RequestMethod,
    ) : WpRequestResult<T>()

    data class RequestExecutionFailed<T>(
        val statusCode: UInt?,
        val redirects: List<WpRedirect>?,
        val reason: RequestExecutionErrorReason,
        val requestUrl: String,
        val requestMethod: RequestMethod,
    ) : WpRequestResult<T>()

    data class MediaFileNotFound<T>(
        val filePath: String
    ) : WpRequestResult<T>()

    data class MediaFileUnreadable<T>(
        val filePath: String
    ) : WpRequestResult<T>()

    data class SiteUrlParsingError<T>(
        val reason: String,
    ) : WpRequestResult<T>()

    data class ResponseParsingError<T>(
        val reason: String,
        val response: String,
        val requestUrl: String,
        val requestMethod: RequestMethod,
    ) : WpRequestResult<T>()

    data class UnknownError<T>(
        val statusCode: UInt,
        val response: String,
        val requestUrl: String,
        val requestMethod: RequestMethod,
    ) : WpRequestResult<T>()

    fun successfulResponse(): T? =
        when (this) {
            is Success -> this.response
            else -> null
        }
}

/**
 * A concise description of a failed request for diagnostics, or `null` when the
 * request succeeded.
 *
 * [policy] decides how much of the request URL and the failed response body the
 * description carries; its defaults write down neither a query parameter's value
 * nor the body's contents.
 *
 * Intended for logs and crash reporting ONLY. Never surface this to users; show
 * a localized, user-facing message instead.
 */
fun WpRequestResult<*>.toLogErrorString(
    policy: RequestErrorLogPolicy = RequestErrorLogPolicy()
): String? = when (this) {
    is WpRequestResult.Success -> null
    is WpRequestResult.WpError ->
        "WpError(code=$errorCode, status=$statusCode, message=$errorMessage, " +
            "method=$requestMethod, url=${policy.redactedUrl(requestUrl)})"
    is WpRequestResult.InvalidHttpStatusCode ->
        "InvalidHttpStatusCode(status=$statusCode, method=$requestMethod, " +
            "url=${policy.redactedUrl(requestUrl)})"
    is WpRequestResult.RequestExecutionFailed ->
        "RequestExecutionFailed(status=$statusCode, reason=$reason, " +
            "method=$requestMethod, url=${policy.redactedUrl(requestUrl)})"
    is WpRequestResult.MediaFileNotFound -> "MediaFileNotFound(path=$filePath)"
    is WpRequestResult.MediaFileUnreadable -> "MediaFileUnreadable(path=$filePath)"
    is WpRequestResult.SiteUrlParsingError -> "SiteUrlParsingError(reason=$reason)"
    is WpRequestResult.ResponseParsingError ->
        "ResponseParsingError(reason=$reason, method=$requestMethod, " +
            "url=${policy.redactedUrl(requestUrl)}${policy.responseField(response)})"
    is WpRequestResult.UnknownError ->
        "UnknownError(status=$statusCode, method=$requestMethod, " +
            "url=${policy.redactedUrl(requestUrl)}${policy.responseField(response)})"
}

private fun RequestErrorLogPolicy.redactedUrl(requestUrl: String): String =
    redactRequestUrlForLog(requestUrl, this.requestUrl)

/**
 * The `response=` portion of a log line, or an empty string when the policy
 * leaves the body out entirely.
 */
private fun RequestErrorLogPolicy.responseField(response: String): String =
    summarizeResponseBodyForLog(response, responseBody)?.let { ", response=$it" }.orEmpty()
