package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.Interceptor
import uniffi.wp_api.AutoDiscoveryAttemptFailure
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.UniffiWpLoginClient
import uniffi.wp_api.VerifyIssuedApplicationPasswordException
import uniffi.wp_api.WpApiApplicationPasswordDetails
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

    /**
     * Sends one authenticated request with an application password the site issued moments ago,
     * to find out whether the site's server passes the `Authorization` header to WordPress.
     *
     * Call this only with freshly issued credentials, right after the authorization callback
     * returns them or the site creates them. Some hosts answer a revoked or wrong password the
     * same way as a blocked header, so the result is meaningless for any other credentials.
     */
    suspend fun verifyIssuedApplicationPassword(
        apiRootUrl: ParsedUrl,
        credentials: WpApiApplicationPasswordDetails
    ): VerifyIssuedApplicationPasswordResult = withContext(dispatcher) {
        try {
            internalClient.verifyIssuedApplicationPassword(apiRootUrl, credentials, null)
            VerifyIssuedApplicationPasswordResult.Verified
        } catch (exception: VerifyIssuedApplicationPasswordException) {
            when (exception) {
                is VerifyIssuedApplicationPasswordException.AuthorizationHeaderBlocked -> {
                    errorLogger?.logFailedRequest(exception.error)
                    VerifyIssuedApplicationPasswordResult.Blocked(exception)
                }
                is VerifyIssuedApplicationPasswordException.Other -> {
                    errorLogger?.logFailedRequest(exception.error)
                    VerifyIssuedApplicationPasswordResult.Other(exception)
                }
            }
        }
    }
}
