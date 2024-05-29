package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import uniffi.wp_api.WpApiException
import uniffi.wp_api.WpAuthentication
import uniffi.wp_api.WpRequestBuilder

class WpApiClient
@Throws(WpApiException::class)
constructor(
    siteUrl: String,
    authentication: WpAuthentication,
    networkHandler: NetworkHandler = WpNetworkHandler(),
    dispatcher: CoroutineDispatcher = Dispatchers.IO
) {
    private val requestBuilder = WpRequestBuilder(siteUrl, authentication)
    val users: UsersEndpoint by lazy {
        WpUsersEndpoint(requestBuilder.users(), networkHandler, dispatcher)
    }
}
