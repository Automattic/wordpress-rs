package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.wp_api.AutoDiscoveryUniffiResult
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.UniffiWpLoginClient
import uniffi.wp_api.WpApiMiddlewarePipeline
import uniffi.wp_api.defaultMiddlewarePipeline

class WpLoginClient(
    requestExecutor: RequestExecutor = WpRequestExecutor(),
    middlewarePipeline: WpApiMiddlewarePipeline = defaultMiddlewarePipeline(),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO
) {
    private val internalClient: UniffiWpLoginClient =
        UniffiWpLoginClient(requestExecutor, middlewarePipeline)

    suspend fun apiDiscovery(siteUrl: String): AutoDiscoveryUniffiResult = withContext(dispatcher) {
        internalClient.apiDiscovery(siteUrl)
    }
}
