package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.UniffiWpApiClient
import uniffi.wp_api.WpApiException
import uniffi.wp_api.WpAuthentication

class WpApiClient
@Throws(WpApiException::class)
constructor(
    siteUrl: ParsedUrl,
    authentication: WpAuthentication,
    private val requestExecutor: RequestExecutor = WpRequestExecutor(),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO
) {
    // Don't expose `WpRequestBuilder` directly so we can control how it's used
    private val requestBuilder by lazy {
        UniffiWpApiClient(siteUrl, authentication, requestExecutor)
    }

    // Provides the _only_ way to execute authenticated requests using our Kotlin wrapper.
    //
    // It makes sure that the errors are wrapped in `WpRequestResult` type instead of forcing
    // clients to try/catch the errors.
    //
    // It'll also help make sure any breaking changes to the API will end up as a compiler error.
    suspend fun <T> request(
        executeRequest: suspend (UniffiWpApiClient) -> T
    ): WpRequestResult<T> = withContext(dispatcher) {
        try {
            WpRequestResult.WpRequestSuccess(data = executeRequest(requestBuilder))
        } catch (exception: WpApiException) {
            when (exception) {
                is WpApiException.InvalidStatusCode -> WpRequestResult.InvalidStatusCode(
                    statusCode = exception.statusCode,
                )
                is WpApiException.RequestExecutionFailed -> WpRequestResult.RequestExecutionFailed(
                    statusCode = exception.statusCode,
                    reason = exception.reason
                )
                is WpApiException.ResponseParsingException -> WpRequestResult.ResponseParsingError(
                    reason = exception.reason,
                    response = exception.response,
                )
                is WpApiException.SiteUrlParsingException -> WpRequestResult.SiteUrlParsingError(
                    reason = exception.reason,
                )
                is WpApiException.UnknownException -> WpRequestResult.UnknownError(
                    statusCode = exception.statusCode,
                    response = exception.response,
                )
                is WpApiException.WpException -> WpRequestResult.WpError(
                    errorCode = exception.errorCode,
                    errorMessage = exception.errorMessage,
                    statusCode = exception.statusCode,
                    response = exception.response,
                )
            }
        }
    }
}
