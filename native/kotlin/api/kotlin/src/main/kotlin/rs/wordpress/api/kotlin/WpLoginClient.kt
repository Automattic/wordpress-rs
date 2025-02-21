package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.wp_api.AutoDiscoveryUniffiResult
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.UniffiWpLoginClient
import uniffi.wp_api.WpLoginClientConfiguration
import uniffi.wp_api.uniffiwploginclientWithConfig

class WpLoginClient {
    private val requestExecutor: RequestExecutor
    private val dispatcher: CoroutineDispatcher
    private val internalClient: UniffiWpLoginClient

    constructor(
        requestExecutor: RequestExecutor = WpRequestExecutor(),
        dispatcher: CoroutineDispatcher = Dispatchers.IO
    ) {
        this.requestExecutor = requestExecutor
        this.dispatcher = dispatcher
        this.internalClient = UniffiWpLoginClient(requestExecutor)
    }

    constructor(
        requestExecutor: RequestExecutor = WpRequestExecutor(),
        dispatcher: CoroutineDispatcher = Dispatchers.IO,
        config: WpLoginClientConfiguration
    ) {
        this.requestExecutor = requestExecutor
        this.dispatcher = dispatcher
        this.internalClient = uniffiwploginclientWithConfig(requestExecutor, config)
    }

    suspend fun apiDiscovery(siteUrl: String): AutoDiscoveryUniffiResult = withContext(dispatcher) {
        internalClient.apiDiscovery(siteUrl)
    }
}
