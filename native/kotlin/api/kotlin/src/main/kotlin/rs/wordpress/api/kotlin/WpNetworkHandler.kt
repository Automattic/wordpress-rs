package rs.wordpress.api.kotlin

import okhttp3.OkHttpClient
import okhttp3.Request
import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse

class WpNetworkHandler : NetworkHandler {
    private val client = OkHttpClient()

    override fun request(request: WpNetworkRequest): WpNetworkResponse {
        val requestBuilder = Request.Builder().url(request.url)
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
