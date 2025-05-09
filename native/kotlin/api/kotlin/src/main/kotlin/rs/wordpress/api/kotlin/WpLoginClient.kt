package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.wp_api.AutoDiscoveryAttemptFailure
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
                    parsedSiteUrl = exception.parsedSiteUrl,
                    findApiRootFailure = exception.findApiRootFailure,
                )

                is AutoDiscoveryAttemptFailure.FetchAndParseApiRoot -> ApiDiscoveryResult.FailureFetchAndParseApiRoot(
                    parsedSiteUrl = exception.parsedSiteUrl,
                    apiRootUrl = exception.apiRootUrl,
                    fetchAndParseApiRootFailure = exception.fetchAndParseApiRootFailure
                )
            }
        }
    }
}
