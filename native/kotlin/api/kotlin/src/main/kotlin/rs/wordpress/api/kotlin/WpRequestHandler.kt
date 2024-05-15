package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.withContext
import uniffi.wp_api.WpApiException
import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse
import uniffi.wp_api.WpRestErrorWrapper

internal class WpRequestHandler(
    private val networkHandler: NetworkHandler,
    private val dispatcher: CoroutineDispatcher
) {
    suspend fun <T> execute(
        request: WpNetworkRequest,
        parser: (response: WpNetworkResponse) -> T
    ): WpRequestResult<T> = withContext(dispatcher) {
        try {
            val response = networkHandler.request(request)
            WpRequestSuccess(data = parser(response))
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
