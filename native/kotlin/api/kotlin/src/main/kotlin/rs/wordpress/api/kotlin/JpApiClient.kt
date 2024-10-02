package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.jetpack_api.JpApiException
import uniffi.jetpack_api.UniffiJetpackClient
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.WpAuthentication

class JpApiClient
@Throws(JpApiException::class)
constructor(
    siteUrl: ParsedUrl,
    authentication: WpAuthentication,
    private val requestExecutor: RequestExecutor = WpRequestExecutor(),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO
) {
    // Don't expose `WpRequestBuilder` directly so we can control how it's used
    private val requestBuilder by lazy {
        UniffiJetpackClient(siteUrl, authentication, requestExecutor)
    }

    // Provides the _only_ way to execute authenticated requests using our Kotlin wrapper.
    //
    // It makes sure that the errors are wrapped in `WpRequestResult` type instead of forcing
    // clients to try/catch the errors.
    //
    // It'll also help make sure any breaking changes to the API will end up as a compiler error.
    suspend fun <T> request(
        executeRequest: suspend (UniffiJetpackClient) -> T
    ): WpRequestResult<T> = withContext(dispatcher) {
        try {
            WpRequestResult.WpRequestSuccess(data = executeRequest(requestBuilder))
        } catch (exception: JpApiException) {
            when (exception) {
                is JpApiException.InvalidHttpStatusCode -> WpRequestResult.InvalidHttpStatusCode(
                    statusCode = exception.statusCode,
                )
                is JpApiException.RequestExecutionFailed -> WpRequestResult.RequestExecutionFailed(
                    statusCode = exception.statusCode,
                    reason = exception.reason
                )
                is JpApiException.ResponseParsingException -> WpRequestResult.ResponseParsingError(
                    reason = exception.reason,
                    response = exception.response,
                )
                is JpApiException.SiteUrlParsingException -> WpRequestResult.SiteUrlParsingError(
                    reason = exception.reason,
                )
                is JpApiException.UnknownException -> WpRequestResult.UnknownError(
                    statusCode = exception.statusCode,
                    response = exception.response,
                )
                is JpApiException.WpException -> WpRequestResult.WpError(
                    errorCode = exception.errorCode,
                    errorMessage = exception.errorMessage,
                    statusCode = exception.statusCode,
                    response = exception.response,
                )
            }
        }
    }
}
