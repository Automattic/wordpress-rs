package rs.wordpress.example.shared.di

import org.koin.dsl.module
import rs.wordpress.example.shared.createWpApiClient
import rs.wordpress.example.shared.ui.users.UserListViewModel

val apiClientModule = module {
    single {
        createWpApiClient()
    }
}

val viewModelModule = module {
    single { UserListViewModel(get()) }
}

fun commonModule() = listOf(
    apiClientModule,
    viewModelModule
)
