package rs.wordpress.example.shared.di

import org.koin.dsl.module
import rs.wordpress.api.kotlin.DebugMiddleware
import rs.wordpress.api.kotlin.EmptyAppNotifier
import rs.wordpress.api.kotlin.NetworkAvailabilityProvider
import rs.wordpress.api.kotlin.WpRequestExecutor
import rs.wordpress.cache.kotlin.WordPressApiCache
import rs.wordpress.example.shared.ui.plugins.PluginListViewModel
import rs.wordpress.example.shared.ui.users.UserListViewModel
import rs.wordpress.example.shared.ui.welcome.WelcomeViewModel
import rs.wordpress.api.kotlin.WpComApiClient
import uniffi.wp_api.WpApiClientDelegate
import uniffi.wp_api.WpApiMiddlewarePipeline
import uniffi.wp_api.WpAuthentication
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import uniffi.wp_mobile.Account
import uniffi.wp_mobile.AccountRepository
import uniffi.wp_mobile.WpService
import java.io.File

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
    single { PluginListViewModel() }
    single { UserListViewModel() }
    single { WelcomeViewModel(get()) }
}

fun createWpService(account: Account.SelfHostedSite, cache: WordPressApiCache): WpService {
    val auth = if (account.siteApiRoot.startsWith("https://public-api.wordpress.com/")) {
        WpAuthentication.Bearer(token = account.password)
    } else {
        wpAuthenticationFromUsernameAndPassword(account.username, account.password)
    }
    return WpService.selfHosted(
        siteUrl = account.domain,
        apiRoot = account.siteApiRoot,
        delegate = WpApiClientDelegate(
            WpAuthenticationProvider.staticWithAuth(auth),
            requestExecutor = WpRequestExecutor(emptyList()),
            middlewarePipeline = WpApiMiddlewarePipeline(listOf(DebugMiddleware())),
            appNotifier = EmptyAppNotifier()
        ),
        cache = cache.cache
    )
}

fun createWpComApiClient(account: Account.WpCom): WpComApiClient {
    return WpComApiClient(
        authProvider = WpAuthenticationProvider.staticWithAuth(
            WpAuthentication.Bearer(token = account.token)
        ),
        interceptors = emptyList()
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
