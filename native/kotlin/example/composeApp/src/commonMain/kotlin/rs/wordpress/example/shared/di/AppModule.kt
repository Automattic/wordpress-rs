package rs.wordpress.example.shared.di

import org.koin.dsl.module
import rs.wordpress.api.kotlin.DebugMiddleware
import rs.wordpress.api.kotlin.EmptyAppNotifier
import rs.wordpress.api.kotlin.NetworkAvailabilityProvider
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestExecutor
import rs.wordpress.cache.kotlin.WordPressApiCache
import rs.wordpress.example.shared.ui.welcome.WelcomeViewModel
import rs.wordpress.api.kotlin.WpComApiClient
import uniffi.wp_api.WpApiClientDelegate
import uniffi.wp_api.WpApiMiddlewarePipeline
import uniffi.wp_api.WpAuthentication
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import uniffi.wp_mobile.Account
import uniffi.wp_mobile.AccountRepository
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.WpComBaseUrl
import uniffi.wp_api.WpComDotOrgApiUrlResolver
import uniffi.wp_mobile.SiteInfo
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
