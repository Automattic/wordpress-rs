package rs.wordpress.api.kotlin
import uniffi.wp_api.CancellationToken
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.WpApiMiddleware
import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse

// Used in a middleware pipeline to print request URLs to the log
class DebugMiddleware : WpApiMiddleware {
    override suspend fun process(
        requestExecutor: RequestExecutor,
        response: WpNetworkResponse,
        request: WpNetworkRequest,
        cancellationToken: CancellationToken?
    ): WpNetworkResponse {
        println("Request: ${request.url()}")
        println("Response:")
        println("\tStatus Code: ${response.statusCode}")
        println("\tHeaders: ${response.responseHeaderMap}")
        return response
    }
}
