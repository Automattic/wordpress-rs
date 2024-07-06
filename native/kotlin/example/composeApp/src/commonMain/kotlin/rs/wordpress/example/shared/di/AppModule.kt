package rs.wordpress.example.shared.di

import org.koin.dsl.module
import rs.wordpress.example.shared.localTestSiteUrl
import rs.wordpress.example.shared.repository.AuthenticationRepository
import rs.wordpress.example.shared.ui.plugins.PluginListViewModel
import rs.wordpress.example.shared.ui.users.UserListViewModel
import rs.wordpress.example.shared.ui.welcome.WelcomeViewModel

val authModule = module {
    single {
        // TODO: Read from test credentials file
        AuthenticationRepository(
            localTestSiteUrl = localTestSiteUrl().siteUrl,
            localTestSiteUsername = "test@example.com",
            // Until this works with the included test credentials, you can grab it from the
            // `test_credentials` file `make test-server` will generate in the root of the repo
            // It's the 3rd line in that file
            localTestSitePassword = "s3N7vlbdrFPDDI3MbyFUvS3P"
        )
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
