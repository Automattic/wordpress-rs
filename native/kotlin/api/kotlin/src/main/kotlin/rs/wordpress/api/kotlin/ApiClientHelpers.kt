package rs.wordpress.api.kotlin

import uniffi.wp_api.WpApiException

fun <T> mapWpApiExceptionToWpRequestResult(apiException: WpApiException): WpRequestResult<T> =
    when (apiException) {
        is WpApiException.InvalidHttpStatusCode -> WpRequestResult.InvalidHttpStatusCode<T>(
            statusCode = apiException.statusCode,
        )

        is WpApiException.RequestExecutionFailed -> WpRequestResult.RequestExecutionFailed<T>(
            statusCode = apiException.statusCode,
            redirects = apiException.redirects,
            reason = apiException.reason
        )

        is WpApiException.MediaFileNotFound -> WpRequestResult.MediaFileNotFound<T>(
            filePath = apiException.filePath
        )

        is WpApiException.ResponseParsingException -> WpRequestResult.ResponseParsingError<T>(
            reason = apiException.reason,
            response = apiException.response,
        )

        is WpApiException.SiteUrlParsingException -> WpRequestResult.SiteUrlParsingError<T>(
            reason = apiException.reason,
        )

        is WpApiException.UnknownException -> WpRequestResult.UnknownError<T>(
            statusCode = apiException.statusCode,
            response = apiException.response,
        )

        is WpApiException.WpException -> WpRequestResult.WpError<T>(
            errorCode = apiException.errorCode,
            errorMessage = apiException.errorMessage,
            statusCode = apiException.statusCode,
            response = apiException.response,
        )
    }
