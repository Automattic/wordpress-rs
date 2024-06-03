package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.withContext
import uniffi.wp_api.WpApiException
import uniffi.wp_api.WpRestErrorWrapper

class WpRequestHandler(
    private val dispatcher: CoroutineDispatcher
) {
    suspend fun <T> execute(
        request: suspend () -> T
    ): WpRequestResult<T> = withContext(dispatcher) {
        try {
            WpRequestSuccess(data = request())
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
