package rs.wordpress.api.kotlin

import okhttp3.OkHttpClient
import okhttp3.Request
import uniffi.wp_api.UserId
import uniffi.wp_api.WpApiClientDelegate
import uniffi.wp_api.WpApiMiddlewarePipeline
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.WpErrorCode
import uniffi.wp_mobile.MockPostService
import uniffi.wp_mobile.WpService
import rs.wordpress.cache.kotlin.WordPressApiCache

const val FIRST_USER_ID: UserId = 1
const val SECOND_USER_ID: UserId = 2
const val FIRST_USER_EMAIL = "test@example.com"
const val SECOND_USER_EMAIL = "themeshaperwp+demos@gmail.com"
const val SECOND_USER_SLUG = "themedemos"
const val NUMBER_OF_USERS = 4
const val NUMBER_OF_PLUGINS = 6
const val HELLO_DOLLY_PLUGIN_SLUG = "hello-dolly/hello"
const val WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS = "classic-widgets"

fun defaultApiClient(): WpApiClient {
    val testCredentials = TestCredentials.INSTANCE
    val authProvider = WpAuthenticationProvider.staticWithUsernameAndPassword(
        username = testCredentials.adminUsername, password = testCredentials.adminPassword
    )
    return WpApiClient(testCredentials.apiRootUrl, authProvider, emptyList())
}

data class TestServiceContext(
    val service: WpService,
    val mockPostService: MockPostService
)

fun createTestServiceContext(): TestServiceContext {
    // Create and migrate cache
    val wordPressApiCache = WordPressApiCache()
    wordPressApiCache.performMigrations()

    val testCredentials = TestCredentials.INSTANCE

    // Create auth provider
    val authProvider = WpAuthenticationProvider.staticWithUsernameAndPassword(
        username = testCredentials.adminUsername,
        password = testCredentials.adminPassword
    )

    val apiRootUrl = testCredentials.apiRootUrl.toString()
    // Extract site URL by removing /wp-json suffix
    val siteUrl = apiRootUrl.removeSuffix("/wp-json")

    // Create self-hosted service
    val service = WpService.selfHosted(
        siteUrl = siteUrl,
        apiRoot = apiRootUrl,
        delegate = WpApiClientDelegate(
            authProvider,
            requestExecutor = WpRequestExecutor(emptyList()),
            middlewarePipeline = WpApiMiddlewarePipeline(emptyList()),
            appNotifier = EmptyAppNotifier()
        ),
        cache = wordPressApiCache.cache
    )

    // Create mock post service with shared cache
    val mockPostService = MockPostService(
        wordPressApiCache.cache,
        siteUrl,
        apiRootUrl
    )

    return TestServiceContext(service, mockPostService)
}

fun <T> WpRequestResult<T>.assertSuccess() {
    assert(this is WpRequestResult.Success)
}

fun <T> WpRequestResult<T>.assertSuccessAndRetrieveData(): T {
    assert(this is WpRequestResult.Success) {
        "Request wasn't successful: $this"
    }
    return (this as WpRequestResult.Success).response
}

fun <T> WpRequestResult<T>.wpErrorCode(): WpErrorCode {
    assert(this is WpRequestResult.WpError)
    return (this as WpRequestResult.WpError).errorCode
}

fun restoreTestServer() {
    OkHttpClient().newCall(
        Request.Builder().url("http://localhost:4000/restore?db=true&plugins=true").build()
    ).execute()
}
