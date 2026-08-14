package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.Interceptor
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.UniffiWpComApiClient
import uniffi.wp_api.WpApiClientDelegate
import uniffi.wp_api.WpApiException
import uniffi.wp_api.WpApiMiddlewarePipeline
import uniffi.wp_api.WpAppNotifier
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.WpComLanguageProvider

class WpComApiClient(
    authProvider: WpAuthenticationProvider,
    private val requestExecutor: RequestExecutor,
    private val appNotifier: WpAppNotifier = EmptyAppNotifier(),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO,
    private val errorLogger: RequestErrorLogger? = null,
    /**
     * Supplies the language every request asks WordPress.com to localize its response to.
     * Leave it `null` to send no locale and let the server choose.
     */
    private val languageProvider: WpComLanguageProvider? = null
) {

    /**
     * Convenience constructor that accepts a list of OkHttp interceptors.
     * Uses [WpRequestExecutor] internally with the provided interceptors.
     */
    // Trailing params are all optional config with defaults; the count is benign here.
    @Suppress("LongParameterList")
    constructor(
        authProvider: WpAuthenticationProvider,
        interceptors: List<Interceptor>,
        networkAvailabilityProvider: NetworkAvailabilityProvider,
        appNotifier: WpAppNotifier = EmptyAppNotifier(),
        dispatcher: CoroutineDispatcher = Dispatchers.IO,
        errorLogger: RequestErrorLogger? = null,
        languageProvider: WpComLanguageProvider? = null
    ) : this(
        authProvider,
        requestExecutor = WpRequestExecutor(interceptors, networkAvailabilityProvider),
        appNotifier,
        dispatcher,
        errorLogger,
        languageProvider
    )

    // Don't expose `WpRequestBuilder` directly so we can control how it's used
    private val requestBuilder by lazy {
        UniffiWpComApiClient(
            WpApiClientDelegate(
                authProvider,
                requestExecutor = requestExecutor,
                middlewarePipeline = WpApiMiddlewarePipeline(emptyList()),
                appNotifier,
                languageProvider = languageProvider
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
        executeRequest: suspend (UniffiWpComApiClient) -> T
    ): WpRequestResult<T> = withContext(dispatcher) {
        val result = try {
            WpRequestResult.Success(response = executeRequest(requestBuilder))
        } catch (exception: WpApiException) {
            mapWpApiExceptionToWpRequestResult<T>(exception)
        }
        errorLogger?.let { logger -> result.toLogErrorString(logger.policy)?.let(logger::logError) }
        result
    }
}
