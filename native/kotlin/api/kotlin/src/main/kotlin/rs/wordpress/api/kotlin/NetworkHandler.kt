package rs.wordpress.api.kotlin

import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse

interface NetworkHandler {
    fun request(request: WpNetworkRequest): WpNetworkResponse
}
