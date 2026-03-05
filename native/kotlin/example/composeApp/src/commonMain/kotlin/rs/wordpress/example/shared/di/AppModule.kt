package rs.wordpress.example.shared.di

import org.koin.dsl.module
import rs.wordpress.api.kotlin.EmptyAppNotifier
import rs.wordpress.api.kotlin.NetworkAvailabilityProvider
import rs.wordpress.api.kotlin.WpRequestExecutor
import rs.wordpress.cache.kotlin.WordPressApiCache
import rs.wordpress.example.TestCredentials
import rs.wordpress.example.shared.localTestSiteUrl
import rs.wordpress.example.shared.repository.AuthenticationRepository
import rs.wordpress.example.shared.ui.plugins.PluginListViewModel
import rs.wordpress.example.shared.ui.postcollection.PostCollectionViewModel
import rs.wordpress.example.shared.ui.postmetadatacollection.PostMetadataCollectionViewModel
import rs.wordpress.example.shared.ui.posttypes.PostTypesViewModel
import rs.wordpress.example.shared.ui.stresstest.StressTestViewModel
import rs.wordpress.example.shared.ui.users.UserListViewModel
import rs.wordpress.example.shared.ui.welcome.WelcomeViewModel
import uniffi.wp_api.WpApiClientDelegate
import uniffi.wp_api.WpApiMiddlewarePipeline
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_mobile.AccountRepository
import uniffi.wp_mobile.MockPostService
import uniffi.wp_mobile.SiteInfo
import uniffi.wp_mobile.WpService
import java.io.File

val authModule = module {
    single {
        AuthenticationRepository(
            accountRepository = getOrNull<AccountRepository>(),
            localTestSiteUrl = localTestSiteUrl().siteUrl,
            localTestSiteUsername = TestCredentials.ADMIN_USERNAME,
            localTestSitePassword = TestCredentials.ADMIN_PASSWORD
        ).apply {
            addTestSiteIfAvailable()
        }
    }
}

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

val mockServiceModule = module {
    single {
        val cache = get<WordPressApiCache>()
        val wpService = get<WpService>()

        // Use the exact same site_url and api_root as WpService
        val siteInfo = wpService.sites().getCurrentSiteInfo()

        val (siteUrl, apiRoot) = when (siteInfo) {
            is SiteInfo.SelfHosted -> siteInfo.siteUrl to siteInfo.apiRoot
            is SiteInfo.WordPressCom -> error("MockPostService requires a self-hosted site")
        }

        MockPostService(
            cache.cache,
            siteUrl,
            apiRoot
        )
    }
}

val selfHostedServiceModule = module {
    single {
        val cache = get<WordPressApiCache>()
        val authRepo = get<AuthenticationRepository>()

        // Get the authenticated site from the repository
        val authenticatedSite = authRepo.authenticatedSiteList().firstOrNull()
        val wpAuth = authenticatedSite?.let { authRepo.authenticationForSite(it) }

        val authProvider = if (wpAuth != null) {
            WpAuthenticationProvider.staticWithAuth(wpAuth)
        } else {
            WpAuthenticationProvider.none()
        }

        val siteUrl = localTestSiteUrl().siteUrl
        val apiRoot = "$siteUrl/wp-json"

        WpService.selfHosted(
            siteUrl = siteUrl,
            apiRoot = apiRoot,
            delegate = WpApiClientDelegate(
                authProvider,
                requestExecutor = WpRequestExecutor(emptyList(), get<NetworkAvailabilityProvider>()),
                middlewarePipeline = WpApiMiddlewarePipeline(emptyList()),
                appNotifier = EmptyAppNotifier()
            ),
            cache = cache.cache
        )
    }
}

val viewModelModule = module {
    // TODO: Need to pass a scoped api client
    single { PluginListViewModel(get(), get()) }
    single { UserListViewModel(get(), get()) }
    single { WelcomeViewModel(get()) }
    single { StressTestViewModel(get(), get(), get()) }
    single { PostCollectionViewModel(get()) }
    single { PostMetadataCollectionViewModel(get()) }
    single { PostTypesViewModel(get()) }
}

fun commonModules() = listOf(
    authModule,
    cacheModule,
    mockServiceModule,
    selfHostedServiceModule,
    viewModelModule
)
