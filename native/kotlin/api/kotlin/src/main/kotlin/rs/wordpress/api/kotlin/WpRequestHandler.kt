package rs.wordpress.api.kotlin

import uniffi.wp_api.WpApiException
import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse
import uniffi.wp_api.WpRestErrorWrapper

class WpRequestHandler(private val networkHandler: NetworkHandler) {
    suspend fun <T> execute(
        request: WpNetworkRequest,
        parser: (response: WpNetworkResponse) -> T
    ): WpRequestResult<T> = try {
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