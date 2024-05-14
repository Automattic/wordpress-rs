package rs.wordpress.api.kotlin

import okhttp3.OkHttpClient
import okhttp3.Request
import uniffi.wp_api.PostListParams
import uniffi.wp_api.PostListResponse
import uniffi.wp_api.WpApiException
import uniffi.wp_api.WpApiHelper
import uniffi.wp_api.WpAuthentication
import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse
import uniffi.wp_api.WpRestErrorWrapper
import uniffi.wp_api.parsePostListResponse

interface NetworkHandler {
    fun request(request: WpNetworkRequest): WpNetworkResponse
}

class WPNetworkHandler: NetworkHandler {
    private val client = OkHttpClient()

    override fun request(request: WpNetworkRequest): WpNetworkResponse {
        val requestBuilder = Request.Builder()
            .url(request.url)
        request.headerMap.forEach { (key, value) ->
            requestBuilder.header(key, value)
        }

        client.newCall(requestBuilder.build()).execute().use { response ->
            return WpNetworkResponse(
                body = response.body?.bytes() ?: ByteArray(0),
                statusCode = response.code.toUShort(),
                headerMap = null
            )
        }
    }
}

class WpRequestHandler(private val networkHandler: NetworkHandler) {
    fun<T> execute(request: WpNetworkRequest, parser: (response: WpNetworkResponse) -> T): WpRequestResult<T> {
        try {
            val response = networkHandler.request(request)
            return WpRequestSuccess(value = parser(response))
        } catch (restException: WpApiException.RestException) {
            return when(restException.restError) {
                is WpRestErrorWrapper.Recognized -> {
                    RecognizedRestError(error = restException.restError.v1)
                }
                is WpRestErrorWrapper.Unrecognized -> {
                    UnrecognizedRestError(error = restException.restError.v1)
                }
            }
        } catch (e: Exception) {
            return UncaughtException(exception = e)
        }
    }
}

class MyClass(
    private val networkHandler: NetworkHandler,
    siteUrl: String,
    authentication: WpAuthentication,
) {
    private val wpApiHelper = WpApiHelper(siteUrl, authentication)

    fun postListRequest(): WpNetworkRequest =
        wpApiHelper.postListRequest(PostListParams())

    fun makePostListRequest(): PostListResponse {
        val wpNetworkRequest = postListRequest()
        return parsePostListResponse(networkHandler.request(wpNetworkRequest))
    }
}
