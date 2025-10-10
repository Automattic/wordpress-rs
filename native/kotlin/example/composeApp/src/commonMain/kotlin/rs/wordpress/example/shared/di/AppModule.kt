package rs.wordpress.example.shared.di

import org.koin.dsl.module
import rs.wordpress.example.TestCredentials
import rs.wordpress.example.shared.localTestSiteUrl
import rs.wordpress.example.shared.repository.AuthenticationRepository
import rs.wordpress.example.shared.ui.plugins.PluginListViewModel
import rs.wordpress.example.shared.ui.users.UserListViewModel
import rs.wordpress.example.shared.ui.welcome.WelcomeViewModel

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

val viewModelModule = module {
    // TODO: Need to pass a scoped api client
    single { PluginListViewModel(get()) }
    single { UserListViewModel(get()) }
    single { WelcomeViewModel(get()) }
}

fun commonModules() = listOf(
    authModule,
    viewModelModule
)
