package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.wp_api.AutoDiscoveryAttemptSuccess
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.UniffiWpLoginClient
import uniffi.wp_api.WpApiMiddlewarePipeline

class WpLoginClient(
    requestExecutor: RequestExecutor = WpRequestExecutor(),
    middlewarePipeline: WpApiMiddlewarePipeline = WpApiMiddlewarePipeline(listOf()),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO
) {
    private val internalClient: UniffiWpLoginClient =
        UniffiWpLoginClient(requestExecutor, middlewarePipeline)

    suspend fun apiDiscovery(siteUrl: String): AutoDiscoveryAttemptSuccess = withContext(dispatcher) {
        internalClient.apiDiscovery(siteUrl)
    }
}
