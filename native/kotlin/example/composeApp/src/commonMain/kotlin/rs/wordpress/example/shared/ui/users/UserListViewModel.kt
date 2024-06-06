package rs.wordpress.example.shared.ui.users

import kotlinx.coroutines.runBlocking
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestSuccess
import uniffi.wp_api.UserWithEditContext

class UserListViewModel(private val apiClient: WpApiClient) {
    fun fetchUsers(): List<UserWithEditContext> {
        val usersResult = runBlocking {
            apiClient.request { requestBuilder ->
                requestBuilder.users().listWithEditContext(params = null)
            }
        }
        return when (usersResult) {
            is WpRequestSuccess -> usersResult.data
            else -> listOf()
        }
    }
}