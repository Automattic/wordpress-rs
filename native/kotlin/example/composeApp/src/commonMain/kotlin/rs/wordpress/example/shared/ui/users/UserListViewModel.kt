package rs.wordpress.example.shared.ui.users

import kotlinx.coroutines.runBlocking
import rs.wordpress.api.kotlin.NetworkAvailabilityProvider
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import rs.wordpress.example.shared.domain.AuthenticatedSite
import rs.wordpress.example.shared.repository.AuthenticationRepository
import uniffi.wp_api.UserListParams
import uniffi.wp_api.UserWithEditContext
import uniffi.wp_api.WpAuthenticationProvider

class UserListViewModel(
    private val authRepository: AuthenticationRepository,
    private val networkAvailabilityProvider: NetworkAvailabilityProvider
) {
    private var apiClient: WpApiClient? = null

    fun setAuthenticatedSite(authenticatedSite: AuthenticatedSite) {
        apiClient = null
        authRepository.authenticationForSite(authenticatedSite)?.let {
            apiClient = WpApiClient(
                wpOrgSiteApiRootUrl = authenticatedSite.apiRootUrl,
                authProvider = WpAuthenticationProvider.staticWithAuth(it),
                interceptors = emptyList(),
                networkAvailabilityProvider = networkAvailabilityProvider
            )
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
                is WpRequestResult.Success -> usersResult.response.data
                else -> listOf()
            }
        }
        return listOf()
    }
}