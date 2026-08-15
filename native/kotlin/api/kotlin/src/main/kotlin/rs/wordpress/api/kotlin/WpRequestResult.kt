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
 * [policy] decides how much of the request URL and the failed response body the
 * description carries; its defaults write down neither a query parameter's value
 * nor the body's contents. See [RequestErrorLogPolicy] for what no policy
 * reaches.
 *
 * Intended for logs and crash reporting ONLY. Never surface this to users; show
 * a localized, user-facing message instead.
 */
fun WpRequestResult<*>.toLogErrorString(
    policy: RequestErrorLogPolicy = RequestErrorLogPolicy.DEFAULT
): String? = when (this) {
    is WpRequestResult.Success -> null
    is WpRequestResult.WpError ->
        // `WpErrorCode`'s variants are payload-free subclasses of `Exception`,
        // so their `toString()` is a fully-qualified class name and a trailing
        // empty message. The variant name alone loses nothing and, with the
        // response's `message` withheld by default, is what names the failure.
        "WpError(code=${errorCode::class.simpleName ?: errorCode}, status=$statusCode" +
            "${policy.responseTextField("message", errorMessage)}, " +
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
        "ResponseParsingError(method=$requestMethod" +
            "${policy.responseTextField("reason", reason)}, " +
            "url=${policy.redactedUrl(requestUrl)}${policy.responseField(response)})"
    is WpRequestResult.UnknownError ->
        "UnknownError(status=$statusCode, method=$requestMethod, " +
            "url=${policy.redactedUrl(requestUrl)}${policy.responseField(response)})"
}
