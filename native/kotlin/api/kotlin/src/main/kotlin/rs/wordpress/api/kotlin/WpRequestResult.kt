package rs.wordpress.api.kotlin

import uniffi.wp_api.WpErrorCode

sealed class WpRequestResult<T> {
    data class WpRequestSuccess<T>(val data: T) : WpRequestResult<T>()
    data class WpError<T>(
        val errorCode: WpErrorCode,
        val errorMessage: String,
        val statusCode: UShort,
        val response: String,
    ) : WpRequestResult<T>()

    data class InvalidHttpStatusCode<T>(
        val statusCode: UShort
    ) : WpRequestResult<T>()

    data class RequestExecutionFailed<T>(
        val statusCode: UShort?,
        val reason: String,
    ) : WpRequestResult<T>()

    data class MediaFileNotFound<T>(
        val filePath: String
    ) : WpRequestResult<T>()

    data class SiteUrlParsingError<T>(
        val reason: String,
    ) : WpRequestResult<T>()

    data class ResponseParsingError<T>(
        val reason: String,
        val response: String,
    ) : WpRequestResult<T>()

    data class UnknownError<T>(
        val statusCode: UShort,
        val response: String,
    ) : WpRequestResult<T>()
}
