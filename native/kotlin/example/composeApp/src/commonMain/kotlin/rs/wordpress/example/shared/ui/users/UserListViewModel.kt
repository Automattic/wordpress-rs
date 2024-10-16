package rs.wordpress.example.shared.ui.users

import kotlinx.coroutines.runBlocking
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import rs.wordpress.example.shared.domain.AuthenticatedSite
import rs.wordpress.example.shared.repository.AuthenticationRepository
import uniffi.wp_api.UserListParams
import uniffi.wp_api.UserWithEditContext

class UserListViewModel(private val authRepository: AuthenticationRepository) {
    private var apiClient: WpApiClient? = null

    fun setAuthenticatedSite(authenticatedSite: AuthenticatedSite) {
        apiClient = null
        authRepository.authenticationForSite(authenticatedSite)?.let {
            apiClient = WpApiClient(siteUrl = authenticatedSite.url, authentication = it)
        }
    }

    fun fetchUsers(): List<UserWithEditContext> {
        apiClient?.let { apiClient ->
            val usersResult = runBlocking {
                apiClient.request { requestBuilder ->
                    requestBuilder.users().listWithEditContext(params = UserListParams())
                }
            }
            return when (usersResult) {
                is WpRequestResult.WpRequestSuccess -> usersResult.data
                else -> listOf()
            }
        }
        return listOf()
    }
}