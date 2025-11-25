package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.Interceptor
import uniffi.wp_api.ApiUrlResolver
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.UniffiWpApiClient
import uniffi.wp_api.WpApiClientDelegate
import uniffi.wp_api.WpApiException
import uniffi.wp_api.WpApiMiddlewarePipeline
import uniffi.wp_api.WpAppNotifier
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.WpOrgSiteApiUrlResolver
import java.net.URL

class WpApiClient(
    apiUrlResolver: ApiUrlResolver,
    authProvider: WpAuthenticationProvider,
    private val requestExecutor: RequestExecutor,
    private val appNotifier: WpAppNotifier = EmptyAppNotifier(),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO
) {
    constructor(
        wpOrgSiteApiRootUrl: URL,
        authProvider: WpAuthenticationProvider,
        requestExecutor: RequestExecutor,
        appNotifier: WpAppNotifier = EmptyAppNotifier(),
        dispatcher: CoroutineDispatcher = Dispatchers.IO
    ) : this(
        apiUrlResolver = WpOrgSiteApiUrlResolver(apiRootUrl = ParsedUrl.parse(wpOrgSiteApiRootUrl.toString())),
        authProvider,
        requestExecutor,
        appNotifier,
        dispatcher
    )

    /**
     * Convenience constructor that accepts a list of OkHttp interceptors.
     * Uses [WpRequestExecutor] internally with the provided interceptors.
     */
    constructor(
        wpOrgSiteApiRootUrl: URL,
        authProvider: WpAuthenticationProvider,
        interceptors: List<Interceptor>,
        appNotifier: WpAppNotifier = EmptyAppNotifier(),
        dispatcher: CoroutineDispatcher = Dispatchers.IO
    ) : this(
        wpOrgSiteApiRootUrl,
        authProvider,
        requestExecutor = WpRequestExecutor(interceptors),
        appNotifier,
        dispatcher
    )

    // Don't expose `WpRequestBuilder` directly so we can control how it's used
    private val requestBuilder by lazy {
        UniffiWpApiClient(
            apiUrlResolver,
            WpApiClientDelegate(
                authProvider,
                requestExecutor = requestExecutor,
                middlewarePipeline = WpApiMiddlewarePipeline(emptyList()),
                appNotifier
            )
        )
    }

    // Provides the _only_ way to execute authenticated requests using our Kotlin wrapper.
    //
    // It makes sure that the errors are wrapped in `WpRequestResult` type instead of forcing
    // clients to try/catch the errors.
    //
    // It'll also help make sure any breaking changes to the API will end up as a compiler error.
    suspend fun <T> request(
        executeRequest: suspend (UniffiWpApiClient) -> T
    ): WpRequestResult<T> = withContext(dispatcher) {
        try {
            WpRequestResult.Success(response = executeRequest(requestBuilder))
        } catch (exception: WpApiException) {
            mapWpApiExceptionToWpRequestResult(exception)
        }
    }
}
