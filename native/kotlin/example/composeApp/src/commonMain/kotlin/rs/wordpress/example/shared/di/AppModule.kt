package rs.wordpress.example.shared.di

import org.koin.dsl.module
import rs.wordpress.api.kotlin.EmptyAppNotifier
import rs.wordpress.api.kotlin.WpRequestExecutor
import rs.wordpress.cache.kotlin.WordPressApiCache
import rs.wordpress.example.TestCredentials
import rs.wordpress.example.shared.localTestSiteUrl
import rs.wordpress.example.shared.repository.AuthenticationRepository
import rs.wordpress.example.shared.ui.plugins.PluginListViewModel
import rs.wordpress.example.shared.ui.postcollection.PostCollectionViewModel
import rs.wordpress.example.shared.ui.stresstest.StressTestViewModel
import rs.wordpress.example.shared.ui.users.UserListViewModel
import rs.wordpress.example.shared.ui.welcome.WelcomeViewModel
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.WpApiClientDelegate
import uniffi.wp_api.WpApiMiddlewarePipeline
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.WpOrgSiteApiUrlResolver
import uniffi.wp_mobile.MockPostService
import uniffi.wp_mobile.WpSelfHostedService
import java.io.File

val authModule = module {
    single {
        AuthenticationRepository(
            localTestSiteUrl = localTestSiteUrl().siteUrl,
            localTestSiteUsername = TestCredentials.ADMIN_USERNAME,
            localTestSitePassword = TestCredentials.ADMIN_PASSWORD
        ).apply {
            // Add test site if credentials are available
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
        val selfHostedService = get<WpSelfHostedService>()

        // Use the exact same site_url and api_root as WpSelfHostedService
        val siteInfo = selfHostedService.sites().getCurrentSiteInfo()

        MockPostService(
            cache.cache,
            siteInfo.siteUrl,
            siteInfo.apiRoot
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

        WpSelfHostedService(
            apiUrlResolver = WpOrgSiteApiUrlResolver(
                apiRootUrl = ParsedUrl.parse("${localTestSiteUrl().siteUrl}/wp-json")
            ),
            delegate = WpApiClientDelegate(
                authProvider,
                requestExecutor = WpRequestExecutor(),
                middlewarePipeline = WpApiMiddlewarePipeline(emptyList()),
                appNotifier = EmptyAppNotifier()
            ),
            cache = cache.cache
        )
    }
}

val viewModelModule = module {
    // TODO: Need to pass a scoped api client
    single { PluginListViewModel(get()) }
    single { UserListViewModel(get()) }
    single { WelcomeViewModel(get()) }
    single { StressTestViewModel(get(), get(), get()) }
    single { PostCollectionViewModel(get()) }
}

fun commonModules() = listOf(
    authModule,
    cacheModule,
    mockServiceModule,
    selfHostedServiceModule,
    viewModelModule
)
