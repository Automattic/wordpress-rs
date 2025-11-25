package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.Interceptor
import uniffi.wp_api.AutoDiscoveryAttemptFailure
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.UniffiWpLoginClient
import uniffi.wp_api.WpApiMiddlewarePipeline

class WpLoginClient(
    requestExecutor: RequestExecutor,
    middlewarePipeline: WpApiMiddlewarePipeline = WpApiMiddlewarePipeline(listOf()),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO
) {

    private val internalClient: UniffiWpLoginClient =
        UniffiWpLoginClient(requestExecutor, middlewarePipeline)

    /**
     * Convenience constructor that accepts a list of OkHttp interceptors.
     * Uses [WpRequestExecutor] internally with the provided interceptors.
     */
    constructor(
        interceptors: List<Interceptor>,
        middlewarePipeline: WpApiMiddlewarePipeline = WpApiMiddlewarePipeline(listOf()),
        dispatcher: CoroutineDispatcher = Dispatchers.IO
    ) : this(
        requestExecutor = WpRequestExecutor(interceptors),
        middlewarePipeline = middlewarePipeline,
        dispatcher = dispatcher
    )

    suspend fun apiDiscovery(
        siteUrl: String
    ): ApiDiscoveryResult = withContext(dispatcher) {
        try {
            val success = internalClient.apiDiscovery(siteUrl)
            ApiDiscoveryResult.Success(success)
        } catch (exception: AutoDiscoveryAttemptFailure) {
            when (exception) {
                is AutoDiscoveryAttemptFailure.ParseSiteUrl -> ApiDiscoveryResult.FailureParseSiteUrl(
                    error = exception.error,
                )
                is AutoDiscoveryAttemptFailure.FindApiRoot -> ApiDiscoveryResult.FailureFindApiRoot(
                    parsedSiteUrl = exception.parsedSiteUrl.toURL(),
                    findApiRootFailure = exception.findApiRootFailure,
                )

                is AutoDiscoveryAttemptFailure.FetchAndParseApiRoot -> ApiDiscoveryResult.FailureFetchAndParseApiRoot(
                    parsedSiteUrl = exception.parsedSiteUrl.toURL(),
                    apiRootUrl = exception.apiRootUrl.toURL(),
                    fetchAndParseApiRootFailure = exception.fetchAndParseApiRootFailure
                )
            }
        }
    }
}
