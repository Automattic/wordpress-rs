package rs.wordpress.api.kotlin

import uniffi.wp_api.RequestExecutionErrorReason
import uniffi.wp_api.RequestMethod
import uniffi.wp_api.WpErrorCode
import uniffi.wp_api.WpRedirect

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
 * Intended for logs and crash reporting ONLY — it may include raw response
 * bodies and request URLs. Never surface this to users; show a localized,
 * user-facing message instead.
 */
fun WpRequestResult<*>.toLogErrorString(): String? = when (this) {
    is WpRequestResult.Success -> null
    is WpRequestResult.WpError ->
        "WpError(code=$errorCode, status=$statusCode, message=$errorMessage, " +
            "method=$requestMethod, url=$requestUrl)"
    is WpRequestResult.InvalidHttpStatusCode ->
        "InvalidHttpStatusCode(status=$statusCode, method=$requestMethod, url=$requestUrl)"
    is WpRequestResult.RequestExecutionFailed ->
        "RequestExecutionFailed(status=$statusCode, reason=$reason, " +
            "method=$requestMethod, url=$requestUrl)"
    is WpRequestResult.MediaFileNotFound -> "MediaFileNotFound(path=$filePath)"
    is WpRequestResult.MediaFileUnreadable -> "MediaFileUnreadable(path=$filePath)"
    is WpRequestResult.SiteUrlParsingError -> "SiteUrlParsingError(reason=$reason)"
    is WpRequestResult.ResponseParsingError ->
        "ResponseParsingError(reason=$reason, method=$requestMethod, " +
            "url=$requestUrl, response=$response)"
    is WpRequestResult.UnknownError ->
        "UnknownError(status=$statusCode, method=$requestMethod, " +
            "url=$requestUrl, response=$response)"
}
