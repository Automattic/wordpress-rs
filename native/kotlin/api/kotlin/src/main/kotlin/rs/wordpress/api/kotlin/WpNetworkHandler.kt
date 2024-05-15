package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse

class WpNetworkHandler(private val dispatcher: CoroutineDispatcher = Dispatchers.IO) :
    NetworkHandler {
    private val client = OkHttpClient()

    override suspend fun request(request: WpNetworkRequest): WpNetworkResponse =
        withContext(dispatcher) {
            val requestBuilder = Request.Builder().url(request.url)
            request.headerMap.forEach { (key, value) ->
                requestBuilder.header(key, value)
            }

            client.newCall(requestBuilder.build()).execute().use { response ->
                return@withContext WpNetworkResponse(
                    body = response.body?.bytes() ?: ByteArray(0),
                    statusCode = response.code.toUShort(),
                    headerMap = null
                )
            }
        }
}
