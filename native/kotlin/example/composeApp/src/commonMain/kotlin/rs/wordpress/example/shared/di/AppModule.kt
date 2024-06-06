package rs.wordpress.example.shared.di

import org.koin.dsl.module
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.example.shared.localTestSiteUrl
import rs.wordpress.example.shared.repository.AuthenticationRepository
import rs.wordpress.example.shared.ui.users.UserListViewModel
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

val localTestSiteApiClientModule = module {
    single {
        WpApiClient(
            siteUrl = localTestSiteUrl().siteUrl,
            authentication = wpAuthenticationFromUsernameAndPassword(
                "test@example.com",
                password = "WpXcVrSWZvPcI1gD9muIOF8l"
            )
        )
    }
}

val authModule = module {
    single { AuthenticationRepository() }
}

val viewModelModule = module {
    single { UserListViewModel(get()) }
}

fun commonModules() = listOf(
    authModule,
    localTestSiteApiClientModule,
    viewModelModule
)
