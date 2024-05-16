package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import uniffi.wp_api.WpApiHelper
import uniffi.wp_api.WpAuthentication

class WpApiClient(
    siteUrl: String,
    authentication: WpAuthentication,
    networkHandler: NetworkHandler = WpNetworkHandler(),
    dispatcher: CoroutineDispatcher = Dispatchers.IO
) {
    private val apiHelper = WpApiHelper(siteUrl, authentication)
    val users: UsersEndpoint by lazy {
        WpUsersEndpoint(apiHelper, networkHandler, dispatcher)
    }
}
