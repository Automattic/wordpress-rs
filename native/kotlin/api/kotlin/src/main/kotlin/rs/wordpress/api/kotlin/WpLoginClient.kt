package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.Interceptor
import uniffi.wp_api.AutoDiscoveryAttemptFailure
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.UniffiWpLoginClient
import uniffi.wp_api.WpApiMiddlewarePipeline

class WpLoginClient @JvmOverloads constructor(
    requestExecutor: RequestExecutor,
    middlewarePipeline: WpApiMiddlewarePipeline = WpApiMiddlewarePipeline(listOf()),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO,
    private val errorLogger: RequestErrorLogger? = null
) {

    private val internalClient: UniffiWpLoginClient =
        UniffiWpLoginClient(requestExecutor, middlewarePipeline)

    /**
     * Convenience constructor that accepts a list of OkHttp interceptors.
     * Uses [WpRequestExecutor] internally with the provided interceptors.
     */
    @JvmOverloads
    constructor(
        interceptors: List<Interceptor> = listOf(),
        networkAvailabilityProvider: NetworkAvailabilityProvider,
        middlewarePipeline: WpApiMiddlewarePipeline = WpApiMiddlewarePipeline(listOf()),
        dispatcher: CoroutineDispatcher = Dispatchers.IO,
        errorLogger: RequestErrorLogger? = null
    ) : this(
        requestExecutor = WpRequestExecutor(interceptors, networkAvailabilityProvider),
        middlewarePipeline = middlewarePipeline,
        dispatcher = dispatcher,
        errorLogger = errorLogger
    )

    suspend fun apiDiscovery(
        siteUrl: String
    ): ApiDiscoveryResult = withContext(dispatcher) {
        try {
            ApiDiscoveryResult.Success(internalClient.apiDiscovery(siteUrl, null))
        } catch (exception: AutoDiscoveryAttemptFailure) {
            errorLogger?.logFailedDiscovery(exception)
            // Pass the failure straight through: it carries its own sealed variants to
            // match on and the localized, translated message (`localizedDescription()`).
            ApiDiscoveryResult.Failure(exception)
        }
    }
}
