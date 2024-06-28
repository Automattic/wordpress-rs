package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.UniffiWpLoginClient
import uniffi.wp_api.UrlDiscoveryResult

class WpLoginClient(
    private val requestExecutor: RequestExecutor = WpRequestExecutor(),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO
) {
    private val internalClient by lazy {
        UniffiWpLoginClient(requestExecutor)
    }

    suspend fun apiDiscovery(siteUrl: String): UrlDiscoveryResult = withContext(dispatcher) {
        internalClient.apiDiscovery(siteUrl)
    }
}