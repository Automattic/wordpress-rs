package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.wp_api.ApiUrlResolver
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.UniffiJetpackApiClient
import uniffi.wp_api.WpApiClientDelegate
import uniffi.wp_api.WpApiException
import uniffi.wp_api.WpApiMiddlewarePipeline
import uniffi.wp_api.WpAppNotifier
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.WpOrgSiteApiUrlResolver

class JetpackApiClient(
    apiUrlResolver: ApiUrlResolver,
    authProvider: WpAuthenticationProvider,
    private val requestExecutor: RequestExecutor = WpRequestExecutor(),
    private val appNotifier: WpAppNotifier = EmptyAppNotifier(),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO
) {
    constructor(
        wpOrgSiteApiRootUrl: ParsedUrl,
        authProvider: WpAuthenticationProvider,
        requestExecutor: RequestExecutor = WpRequestExecutor(),
        appNotifier: WpAppNotifier = EmptyAppNotifier(),
        dispatcher: CoroutineDispatcher = Dispatchers.IO
    ) : this(
        apiUrlResolver = WpOrgSiteApiUrlResolver(apiRootUrl = wpOrgSiteApiRootUrl),
        authProvider,
        requestExecutor,
        appNotifier,
        dispatcher
    )

    // Don't expose `WpRequestBuilder` directly so we can control how it's used
    private val requestBuilder by lazy {
        UniffiJetpackApiClient(
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
        executeRequest: suspend (UniffiJetpackApiClient) -> T
    ): WpRequestResult<T> = withContext(dispatcher) {
        try {
            WpRequestResult.Success(response = executeRequest(requestBuilder))
        } catch (exception: WpApiException) {
            mapWpApiExceptionToWpRequestResult(exception)
        }
    }
}
