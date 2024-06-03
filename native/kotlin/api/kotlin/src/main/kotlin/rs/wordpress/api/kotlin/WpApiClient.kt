package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.WpApiException
import uniffi.wp_api.WpAuthentication
import uniffi.wp_api.WpRequestBuilder

class WpApiClient
@Throws(WpApiException::class)
constructor(
    siteUrl: String,
    authentication: WpAuthentication,
    requestExecutor: RequestExecutor = WpRequestExecutor(),
    dispatcher: CoroutineDispatcher = Dispatchers.IO
) {
    val requestBuilder = WpRequestBuilder(siteUrl, authentication, requestExecutor)
    val requestHandler = WpRequestHandler(dispatcher)
}
