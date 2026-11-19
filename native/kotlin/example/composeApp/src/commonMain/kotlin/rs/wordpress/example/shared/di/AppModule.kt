package rs.wordpress.example.shared.di

import org.koin.dsl.module
import rs.wordpress.api.kotlin.DebugMiddleware
import rs.wordpress.api.kotlin.EmptyAppNotifier
import rs.wordpress.api.kotlin.NetworkAvailabilityProvider
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestExecutor
import rs.wordpress.cache.kotlin.WordPressApiCache
import rs.wordpress.example.shared.ui.reader.ReaderSession
import rs.wordpress.example.shared.ui.welcome.WelcomeViewModel
import rs.wordpress.api.kotlin.WpComApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import uniffi.wp_api.WpApiClientDelegate
import uniffi.wp_api.WpApiMiddlewarePipeline
import uniffi.wp_api.WpAuthentication
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import uniffi.wp_mobile.Account
import uniffi.wp_mobile.AccountRepository
import uniffi.wp_mobile.ReaderService
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.ReaderSavedSource
import uniffi.wp_api.WpComBaseUrl
import uniffi.wp_api.WpComDotOrgApiUrlResolver
import uniffi.wp_mobile.SiteInfo
import uniffi.wp_mobile_cache.ReaderOwner
import uniffi.wp_mobile.WpService
import java.io.File
import java.net.URI

val cacheModule = module {
    single {
        // Create temporary file for disk-based database testing
        val tempFile = File.createTempFile("wordpress_cache_", ".db")
        tempFile.deleteOnExit()
        println("📁 Using disk-based DB: ${tempFile.absolutePath}")

        WordPressApiCache(tempFile.toPath()).apply {
            performMigrations()
        }
    }
}

val viewModelModule = module {
    single { WelcomeViewModel(get()) }
}

fun createWpService(
    account: Account.SelfHostedSite,
    cache: WordPressApiCache,
    networkAvailabilityProvider: NetworkAvailabilityProvider = NetworkAvailabilityProvider { true }
): WpService {
    val auth = if (account.siteApiRoot.startsWith("https://public-api.wordpress.com/")) {
        WpAuthentication.Bearer(token = account.password)
    } else {
        wpAuthenticationFromUsernameAndPassword(account.username, account.password)
    }
    return WpService(
        siteInfo = SiteInfo.SelfHosted(
            siteUrl = ParsedUrl.parse(account.domain),
            apiRoot = ParsedUrl.parse(account.siteApiRoot)
        ),
        delegate = WpApiClientDelegate(
            WpAuthenticationProvider.staticWithAuth(auth),
            requestExecutor = WpRequestExecutor(emptyList(), networkAvailabilityProvider),
            middlewarePipeline = WpApiMiddlewarePipeline(listOf(DebugMiddleware())),
            appNotifier = EmptyAppNotifier()
        ),
        cache = cache.cache
    )
}

fun createWpService(
    account: Account.WpCom,
    cache: WordPressApiCache,
    networkAvailabilityProvider: NetworkAvailabilityProvider = NetworkAvailabilityProvider { true }
): WpService {
    val siteId = account.siteApiRoot.toULong()
    return WpService(
        siteInfo = SiteInfo.WordPressCom(
            siteId = siteId
        ),
        delegate = WpApiClientDelegate(
            WpAuthenticationProvider.staticWithAuth(
                WpAuthentication.Bearer(token = account.token)
            ),
            requestExecutor = WpRequestExecutor(emptyList(), networkAvailabilityProvider),
            middlewarePipeline = WpApiMiddlewarePipeline(listOf(DebugMiddleware())),
            appNotifier = EmptyAppNotifier()
        ),
        cache = cache.cache
    )
}

fun createWpApiClient(
    account: Account.SelfHostedSite,
    networkAvailabilityProvider: NetworkAvailabilityProvider = NetworkAvailabilityProvider { true }
): WpApiClient {
    val auth = if (account.siteApiRoot.startsWith("https://public-api.wordpress.com/")) {
        WpAuthentication.Bearer(token = account.password)
    } else {
        wpAuthenticationFromUsernameAndPassword(account.username, account.password)
    }
    return WpApiClient(
        wpOrgSiteApiRootUrl = URI(account.siteApiRoot).toURL(),
        authProvider = WpAuthenticationProvider.staticWithAuth(auth),
        interceptors = emptyList(),
        networkAvailabilityProvider = networkAvailabilityProvider
    )
}

fun createWpApiClient(
    account: Account.WpCom,
    networkAvailabilityProvider: NetworkAvailabilityProvider = NetworkAvailabilityProvider { true }
): WpApiClient {
    return WpApiClient(
        apiUrlResolver = WpComDotOrgApiUrlResolver(
            siteId = account.siteApiRoot,
            baseUrl = WpComBaseUrl.Production
        ),
        authProvider = WpAuthenticationProvider.staticWithAuth(
            WpAuthentication.Bearer(token = account.token)
        ),
        requestExecutor = WpRequestExecutor(emptyList(), networkAvailabilityProvider)
    )
}

fun createWpComApiClient(
    account: Account.WpCom,
    networkAvailabilityProvider: NetworkAvailabilityProvider = NetworkAvailabilityProvider { true }
): WpComApiClient {
    return WpComApiClient(
        authProvider = WpAuthenticationProvider.staticWithAuth(
            WpAuthentication.Bearer(token = account.token)
        ),
        interceptors = emptyList(),
        networkAvailabilityProvider = networkAvailabilityProvider
    )
}

/** Where the Reader keeps its cache and each account's user ID, across launches. */
private val readerDirectory = File(System.getProperty("java.io.tmpdir"), "reader-saved-posts")

/** Where the Reader keeps saved posts' images, for reading them with no connection. */
val readerOfflineImageDirectory: File = File(readerDirectory, "offline-images")

/** How much room saved posts' images may take: enough for a few dozen posts. */
const val READER_OFFLINE_IMAGE_BYTES: ULong = 50_000_000uL

/**
 * The Reader's cache: a file that lasts across launches, unlike the shared one in [cacheModule],
 * so the Reader shows its streams and saved posts after a relaunch, offline. Every reader's data
 * is in it, each under its owner.
 */
val readerCache: WordPressApiCache by lazy {
    readerDirectory.mkdirs()
    WordPressApiCache(File(readerDirectory, "cache.db").toPath()).apply {
        performMigrations()
    }
}

/**
 * The account's WordPress.com user ID: from `/me` the first time, and remembered after that, as an
 * app would have it, so the Reader opens offline.
 */
private suspend fun wpComUserId(account: Account.WpCom, wpComApiClient: WpComApiClient): ULong {
    val file = File(readerDirectory, "user-${account.id}")
    file.takeIf { it.exists() }?.readText()?.toULongOrNull()?.let { return it }
    val userId = when (val result = wpComApiClient.request { it.me().get() }) {
        is WpRequestResult.Success -> result.response.data.id
        else -> error("Couldn't load the account's user ID: $result")
    }
    readerDirectory.mkdirs()
    file.writeText(userId.toString())
    return userId
}

/** Each reader's Reader, for as long as the app runs: a reader has one service, which every screen shares. */
private val readerSessions = mutableMapOf<ReaderOwner, ReaderSession>()

private fun readerSession(
    owner: ReaderOwner,
    label: String,
    authentication: WpAuthentication,
    networkAvailabilityProvider: NetworkAvailabilityProvider
): ReaderSession = synchronized(readerSessions) {
    readerSessions.getOrPut(owner) {
        val service = ReaderService(
            delegate = WpApiClientDelegate(
                WpAuthenticationProvider.staticWithAuth(authentication),
                requestExecutor = WpRequestExecutor(emptyList(), networkAvailabilityProvider),
                middlewarePipeline = WpApiMiddlewarePipeline(listOf(DebugMiddleware())),
                appNotifier = EmptyAppNotifier()
            ),
            cache = readerCache.cache,
            owner = owner,
            savedSource = ReaderSavedSource.ANDROID
        )
        ReaderSession(service, label, isSignedIn = owner is ReaderOwner.Account)
    }
}

/**
 * The Reader of the reader who isn't signed in to WordPress.com, kept in [readerCache]. Its client
 * sends no authentication, and what the reader follows and saves stays on this device.
 */
fun signedOutReaderSession(
    networkAvailabilityProvider: NetworkAvailabilityProvider = NetworkAvailabilityProvider { true }
): ReaderSession = readerSession(
    owner = ReaderOwner.SignedOut,
    label = "nobody (signed out)",
    authentication = WpAuthentication.None,
    networkAvailabilityProvider = networkAvailabilityProvider
)

/** The account's Reader, kept in [readerCache] under the account's WordPress.com user ID. */
suspend fun accountReaderSession(
    account: Account.WpCom,
    wpComApiClient: WpComApiClient,
    networkAvailabilityProvider: NetworkAvailabilityProvider = NetworkAvailabilityProvider { true }
): ReaderSession {
    val userId = wpComUserId(account, wpComApiClient)
    return readerSession(
        owner = ReaderOwner.Account(userId),
        label = "WordPress.com user $userId",
        authentication = WpAuthentication.Bearer(token = account.token),
        networkAvailabilityProvider = networkAvailabilityProvider
    )
}

fun commonModules(accountRepository: AccountRepository): List<org.koin.core.module.Module> {
    val accountModule = module {
        single { accountRepository }
    }

    return listOf(
        accountModule,
        cacheModule,
        viewModelModule
    )
}
