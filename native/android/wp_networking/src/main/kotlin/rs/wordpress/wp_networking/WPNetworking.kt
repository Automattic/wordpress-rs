package rs.wordpress.wp_networking

import uniffi.wp_api.WpApiInterface
import uniffi.wp_api.WpAuthentication
import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse
import uniffi.wp_api.WpNetworkingInterface
import uniffi.wp_networking.wpApiWithCustomNetworking

fun wpApi(): WpApiInterface {
    val authentication = WpAuthentication(authToken = "ZGVtbzo0alp6IGNid2UgOXdkVCBFcE1kIGpQVDQgTkZCRg==")
    val siteUrl = "https://clever-coffee-nut.jurassic.ninja"
    return wpApiWithCustomNetworking(siteUrl, authentication, WPNetworking())
}

class WPNetworking: WpNetworkingInterface {
    override fun request(request: WpNetworkRequest): WpNetworkResponse {
        TODO("Not yet implemented")
    }
}
