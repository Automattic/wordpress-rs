package rs.wordpress.api.kotlin

import okhttp3.OkHttpClient
import okhttp3.Request
import uniffi.wp_api.PostListParams
import uniffi.wp_api.PostListResponse
import uniffi.wp_api.WpApiHelper
import uniffi.wp_api.WpAuthentication
import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse
import uniffi.wp_api.parsePostListResponse

class MyClass(siteUrl: String, authentication: WpAuthentication) {
    private val wpApiHelper = WpApiHelper(siteUrl, authentication)
    private val client = OkHttpClient()

    fun postListRequest(): WpNetworkRequest =
        wpApiHelper.postListRequest(PostListParams())

    fun makePostListRequest(): PostListResponse {
        val wpNetworkRequest = postListRequest()
        return parsePostListResponse(request(wpNetworkRequest))
    }

    private fun request(wpNetworkRequest: WpNetworkRequest): WpNetworkResponse {
        val requestBuilder = Request.Builder()
            .url(wpNetworkRequest.url)
        wpNetworkRequest.headerMap.forEach { (key, value) ->
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
