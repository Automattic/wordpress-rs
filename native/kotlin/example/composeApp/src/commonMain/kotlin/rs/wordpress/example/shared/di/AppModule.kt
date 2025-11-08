package rs.wordpress.example.shared.di

import org.koin.dsl.module
import rs.wordpress.api.kotlin.EmptyAppNotifier
import rs.wordpress.api.kotlin.WpRequestExecutor
import rs.wordpress.cache.kotlin.WordPressApiCache
import rs.wordpress.example.TestCredentials
import rs.wordpress.example.shared.localTestSiteUrl
import rs.wordpress.example.shared.repository.AuthenticationRepository
import rs.wordpress.example.shared.ui.plugins.PluginListViewModel
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
        WordPressApiCache().apply {
            performMigrations()
        }
    }
}

val mockServiceModule = module {
    single {
        val cache = get<WordPressApiCache>()
        val siteUrl = localTestSiteUrl().siteUrl

        MockPostService(
            cache.cache,
            siteUrl
        )
    }
}

val selfHostedServiceModule = module {
    single {
        val cache = get<WordPressApiCache>()
        val siteUrl = localTestSiteUrl().siteUrl

        // Use no auth for stress test - we're only using MockPostService to insert data
        val authProvider = WpAuthenticationProvider.none()

        WpSelfHostedService(
            apiUrlResolver = WpOrgSiteApiUrlResolver(
                apiRootUrl = ParsedUrl.parse(siteUrl)
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
}

fun commonModules() = listOf(
    authModule,
    cacheModule,
    mockServiceModule,
    selfHostedServiceModule,
    viewModelModule
)
