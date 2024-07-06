package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.UniffiWpLoginClient
import uniffi.wp_api.UrlDiscoveryException
import uniffi.wp_api.UrlDiscoverySuccess

class WpLoginClient(
    private val requestExecutor: RequestExecutor = WpRequestExecutor(),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO
) {
    private val internalClient by lazy {
        UniffiWpLoginClient(requestExecutor)
    }

    suspend fun apiDiscovery(siteUrl: String): Result<UrlDiscoverySuccess> = withContext(dispatcher) {
        try {
            Result.success(internalClient.apiDiscovery(siteUrl))
        } catch (e: UrlDiscoveryException) {
            Result.failure(e)
        }
    }
}
