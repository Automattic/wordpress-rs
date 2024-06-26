package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.wp_api.FindApiUrlsException
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.UniffiWpLoginClient
import uniffi.wp_api.WpRestApiUrls

class WpLoginClient(
    private val requestExecutor: RequestExecutor = WpRequestExecutor(),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO
) {
    private val internalClient by lazy {
        UniffiWpLoginClient(requestExecutor)
    }

    suspend fun apiDiscovery(siteUrl: String): Result<WpRestApiUrls> = withContext(dispatcher) {
        try {
            Result.success(internalClient.apiDiscovery(siteUrl))
        } catch (e: FindApiUrlsException) {
            Result.failure(e)
        }
    }
}