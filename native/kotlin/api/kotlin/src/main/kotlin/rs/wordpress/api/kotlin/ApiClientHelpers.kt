package rs.wordpress.api.kotlin

import uniffi.wp_api.WpApiException

private const val REQUEST_ERROR_LOG_TAG = "WpRequestError"

/**
 * Logs a failed request via `System.err`, which is redirected to logcat on
 * Android (the same mechanism [DebugMiddleware] relies on). No-op on success
 * or when [enabled] is `false`.
 */
internal fun WpRequestResult<*>.logErrorIfNeeded(enabled: Boolean) {
    if (!enabled) return
    toLogErrorString()?.let { System.err.println("$REQUEST_ERROR_LOG_TAG: $it") }
}

fun <T> mapWpApiExceptionToWpRequestResult(apiException: WpApiException): WpRequestResult<T> =
    when (apiException) {
        is WpApiException.InvalidHttpStatusCode -> WpRequestResult.InvalidHttpStatusCode<T>(
            statusCode = apiException.statusCode,
            requestUrl = apiException.requestUrl,
            requestMethod = apiException.requestMethod,
        )

        is WpApiException.RequestExecutionFailed -> WpRequestResult.RequestExecutionFailed<T>(
            statusCode = apiException.statusCode,
            redirects = apiException.redirects,
            reason = apiException.reason,
            requestUrl = apiException.requestUrl,
            requestMethod = apiException.requestMethod,
        )

        is WpApiException.MediaFileNotFound -> WpRequestResult.MediaFileNotFound<T>(
            filePath = apiException.filePath
        )

        is WpApiException.ResponseParsingException -> WpRequestResult.ResponseParsingError<T>(
            reason = apiException.reason,
            response = apiException.response,
            requestUrl = apiException.requestUrl,
            requestMethod = apiException.requestMethod,
        )

        is WpApiException.SiteUrlParsingException -> WpRequestResult.SiteUrlParsingError<T>(
            reason = apiException.reason,
        )

        is WpApiException.UnknownException -> WpRequestResult.UnknownError<T>(
            statusCode = apiException.statusCode,
            response = apiException.response,
            requestUrl = apiException.requestUrl,
            requestMethod = apiException.requestMethod,
        )

        is WpApiException.WpException -> WpRequestResult.WpError<T>(
            errorCode = apiException.errorCode,
            errorMessage = apiException.errorMessage,
            statusCode = apiException.statusCode,
            response = apiException.response,
            requestUrl = apiException.requestUrl,
            requestMethod = apiException.requestMethod,
        )
    }
