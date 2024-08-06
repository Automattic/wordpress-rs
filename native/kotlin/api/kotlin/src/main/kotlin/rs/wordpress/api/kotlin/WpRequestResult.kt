package rs.wordpress.api.kotlin

import uniffi.wp_api.WpErrorCode

sealed class WpRequestResult<T> {
    class WpRequestSuccess<T>(val data: T) : WpRequestResult<T>()
    class WpError<T>(
        val errorCode: WpErrorCode,
        val errorMessage: String,
        val statusCode: UShort,
        val response: String,
    ) : WpRequestResult<T>()

    class InvalidStatusCode<T>(
        val statusCode: UShort
    ) : WpRequestResult<T>()

    class RequestExecutionFailed<T>(
        val statusCode: UShort?,
        val reason: String,
    ) : WpRequestResult<T>()

    class SiteUrlParsingError<T>(
        val reason: String,
    ) : WpRequestResult<T>()

    class ResponseParsingError<T>(
        val reason: String,
        val response: String,
    ) : WpRequestResult<T>()

    class UnknownError<T>(
        val statusCode: UShort,
        val response: String,
    ) : WpRequestResult<T>()
}
