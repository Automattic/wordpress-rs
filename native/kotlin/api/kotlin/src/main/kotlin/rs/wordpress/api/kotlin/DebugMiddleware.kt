package rs.wordpress.api.kotlin
import uniffi.wp_api.RequestContext
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.WpApiMiddleware
import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse

// Used in a middleware pipeline to print request details to the log
class DebugMiddleware : WpApiMiddleware {
    override suspend fun process(
        requestExecutor: RequestExecutor,
        response: WpNetworkResponse,
        request: WpNetworkRequest,
        context: RequestContext?
    ): WpNetworkResponse {
        val tag = "WpDebug"
        val url = request.url()
        val method = request.method()
        val requestHeaders = request.headerMap().toMap()
        val responseHeaders = response.responseHeaderMap.toMap()
        val body = String(response.body)

        log(tag, "===== REQUEST =====")
        log(tag, "$method $url")
        log(tag, "--- Request Headers ---")
        requestHeaders.forEach { (key, values) ->
            values.forEach { value ->
                log(tag, "  $key: $value")
            }
        }
        log(tag, "===== RESPONSE =====")
        log(tag, "Status: ${response.statusCode}")
        log(tag, "--- Response Headers ---")
        responseHeaders.forEach { (key, values) ->
            values.forEach { value ->
                log(tag, "  $key: $value")
            }
        }
        log(tag, "--- Response Body ---")
        log(tag, body)
        log(tag, "====================")
        return response
    }

    private fun log(tag: String, message: String) {
        // Use System.err which reliably appears in logcat
        System.err.println("$tag: $message")
    }
}
