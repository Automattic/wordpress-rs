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
            val success = internalClient.apiDiscovery(siteUrl, null)
            ApiDiscoveryResult.Success(success)
        } catch (exception: AutoDiscoveryAttemptFailure) {
            errorLogger?.logFailedDiscovery(exception)
            // Retain the whole `AutoDiscoveryAttemptFailure` rather than destructuring it
            // away: it is the only discovery type that carries a localized, translated
            // message (`localizedDescription()`), so dropping it here would discard the
            // actionable reason for the failure.
            when (exception) {
                is AutoDiscoveryAttemptFailure.ParseSiteUrl ->
                    ApiDiscoveryResult.FailureParseSiteUrl(exception)
                is AutoDiscoveryAttemptFailure.FindApiRoot ->
                    ApiDiscoveryResult.FailureFindApiRoot(exception)
                is AutoDiscoveryAttemptFailure.FetchAndParseApiRoot ->
                    ApiDiscoveryResult.FailureFetchAndParseApiRoot(exception)
            }
        }
    }
}
