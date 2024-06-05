package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.WpApiException
import uniffi.wp_api.WpAuthentication
import uniffi.wp_api.WpRequestBuilder
import uniffi.wp_api.WpRestErrorWrapper

class WpApiClient
@Throws(WpApiException::class)
constructor(
    siteUrl: String,
    authentication: WpAuthentication,
    private val requestExecutor: RequestExecutor = WpRequestExecutor(),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO
) {
    // Don't expose `WpRequestBuilder` directly so we can control how it's used
    private val requestBuilder by lazy {
        WpRequestBuilder(siteUrl, authentication, requestExecutor)
    }

    // Provides the _only_ way to execute requests using our Kotlin wrapper.
    //
    // It makes sure that the errors are wrapped in `WpRequestResult` type instead of forcing
    // clients to try/catch the errors.
    //
    // It'll also help make sure any breaking changes to the API will end up as a compiler error.
    suspend fun <T> request(
        buildRequest: suspend (WpRequestBuilder) -> T
    ): WpRequestResult<T> = withContext(dispatcher) {
        try {
            WpRequestSuccess(data = buildRequest(requestBuilder))
        } catch (restException: WpApiException.RestException) {
            when (restException.restError) {
                is WpRestErrorWrapper.Recognized -> {
                    RecognizedRestError(error = restException.restError.v1)
                }

                is WpRestErrorWrapper.Unrecognized -> {
                    UnrecognizedRestError(error = restException.restError.v1)
                }
            }
        }
    }
}
