package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.Interceptor
import rs.wordpress.cache.kotlin.WordPressApiCache
import uniffi.wp_api.ReaderSavedSource
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.UniffiWpComApiClient
import uniffi.wp_api.WpApiClientDelegate
import uniffi.wp_api.WpApiException
import uniffi.wp_api.WpApiMiddlewarePipeline
import uniffi.wp_api.WpAppNotifier
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.WpComLanguageProvider
import uniffi.wp_mobile.NotificationService
import uniffi.wp_mobile.ReaderService
import uniffi.wp_mobile_cache.ReaderOwner

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

    private val delegate by lazy {
        WpApiClientDelegate(
            authProvider,
            requestExecutor = requestExecutor,
            middlewarePipeline = WpApiMiddlewarePipeline(emptyList()),
            appNotifier,
            languageProvider = languageProvider
        )
    }

    // Don't expose `WpRequestBuilder` directly so we can control how it's used
    private val requestBuilder by lazy { UniffiWpComApiClient(delegate) }

    /**
     * Creates the Reader service for one reader: an account, or the device when no one is signed
     * in. It keeps streams, posts, comments, follows and saves in [cache], shows a change at once,
     * and sends it when it can.
     *
     * For a signed-out reader, pass [ReaderOwner.SignedOut] on a client whose `authProvider` sends
     * no authentication. What that reader follows and saves stays on the device.
     *
     * The cache's migrations must have run. [savedSource] is the app, which is recorded with each
     * save. Create one per reader and share it: it sends its saves one at a time, and a second
     * instance would send them alongside it. Its other changes go in the account's one queue in
     * [cache], which the account's notifications service shares.
     */
    fun makeReaderService(
        cache: WordPressApiCache,
        owner: ReaderOwner,
        savedSource: ReaderSavedSource
    ): ReaderService = ReaderService(delegate, cache.cache, owner, savedSource)

    /**
     * Creates the notifications service for one account. It keeps the account's notes in [cache],
     * syncs the newest by comparing hashes, and loads older ones a page at a time. Its syncs send
     * only GET requests.
     *
     * Likes and follows made from a note go in the account's one queue in [cache], which the
     * account's Reader service shares, so they're sent whichever of the two exist. The cache's
     * migrations must have run. Create one per account and share it.
     */
    fun makeNotificationService(
        cache: WordPressApiCache,
        userId: ULong
    ): NotificationService = NotificationService(delegate, cache.cache, userId)

    // Provides the _only_ way to execute authenticated requests using our Kotlin wrapper.
    //
    // It makes sure that the errors are wrapped in `WpRequestResult` type instead of forcing
    // clients to try/catch the errors.
    //
    // It'll also help make sure any breaking changes to the API will end up as a compiler error.
    suspend fun <T> request(
        executeRequest: suspend (UniffiWpComApiClient) -> T
    ): WpRequestResult<T> = withContext(dispatcher) {
        try {
            WpRequestResult.Success(response = executeRequest(requestBuilder))
        } catch (exception: WpApiException) {
            errorLogger?.logFailedRequest(exception)
            mapWpApiExceptionToWpRequestResult<T>(exception)
        }
    }
}
