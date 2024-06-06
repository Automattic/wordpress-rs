package rs.wordpress.example.shared.di

import org.koin.dsl.module
import rs.wordpress.example.shared.localTestSiteUrl
import rs.wordpress.example.shared.repository.AuthenticationRepository
import rs.wordpress.example.shared.ui.users.UserListViewModel
import rs.wordpress.example.shared.ui.welcome.WelcomeViewModel

val authModule = module {
    single {
        // TODO: Read from test credentials file
        AuthenticationRepository(
            localTestSiteUrl = localTestSiteUrl().siteUrl,
            localTestSiteUsername = "test@example.com",
            localTestSitePassword = "WpXcVrSWZvPcI1gD9muIOF8l"
        )
    }
}

val viewModelModule = module {
    // TODO: Need to pass a scoped api client
    single { UserListViewModel(get()) }
    single { WelcomeViewModel(get()) }
}

fun commonModules() = listOf(
    authModule,
    viewModelModule
)
