package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.UniffiWpComApiClient
import uniffi.wp_api.WpApiClientDelegate
import uniffi.wp_api.WpApiException
import uniffi.wp_api.WpApiMiddlewarePipeline
import uniffi.wp_api.WpAppNotifier
import uniffi.wp_api.WpAuthenticationProvider

class WpComApiClient(
    authProvider: WpAuthenticationProvider,
    private val requestExecutor: RequestExecutor = WpRequestExecutor(),
    private val appNotifier: WpAppNotifier = EmptyAppNotifier(),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO
) {
    // Don't expose `WpRequestBuilder` directly so we can control how it's used
    private val requestBuilder by lazy {
        UniffiWpComApiClient(
            WpApiClientDelegate(
                authProvider,
                requestExecutor = requestExecutor,
                middlewarePipeline = WpApiMiddlewarePipeline(emptyList()),
                appNotifier
            )
        )
    }

    // Provides the _only_ way to execute authenticated requests using our Kotlin wrapper.
    //
    // It makes sure that the errors are wrapped in `WpRequestResult` type instead of forcing
    // clients to try/catch the errors.
    //
    // It'll also help make sure any breaking changes to the API will end up as a compiler error.
    suspend fun <T> request(
        executeRequest: suspend (UniffiWpComApiClient) -> T
    ): WpRequestResult<T> = withContext(dispatcher) {
        try {
            WpRequestResult.WpRequestSuccess(data = executeRequest(requestBuilder))
        } catch (exception: WpApiException) {
            mapWpApiExceptionToWpRequestResult(exception)
        }
    }
}
