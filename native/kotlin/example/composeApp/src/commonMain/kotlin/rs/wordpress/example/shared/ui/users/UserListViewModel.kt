package rs.wordpress.example.shared.ui.users

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import uniffi.wp_api.UserListParams
import uniffi.wp_api.UserWithEditContext
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import uniffi.wp_mobile.Account
import java.net.URI

class UserListViewModel {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private var apiClient: WpApiClient? = null

    private val _users = MutableStateFlow<List<UserWithEditContext>>(emptyList())
    val users: StateFlow<List<UserWithEditContext>> = _users.asStateFlow()

    fun setAccount(account: Account) {
        apiClient = null
        _users.value = emptyList()
        when (account) {
            is Account.SelfHostedSite -> {
                apiClient = WpApiClient(
                    wpOrgSiteApiRootUrl = URI(account.siteApiRoot).toURL(),
                    authProvider = WpAuthenticationProvider.staticWithAuth(
                        wpAuthenticationFromUsernameAndPassword(account.username, account.password)
                    ),
                    interceptors = emptyList()
                )
                loadUsers()
            }
            is Account.WpCom -> {
                // WP.com accounts not yet supported in this view model
            }
        }
    }

    private fun loadUsers() {
        viewModelScope.launch(Dispatchers.IO) {
            apiClient?.let { client ->
                val usersResult = client.request { requestBuilder ->
                    requestBuilder.users().listWithEditContext(params = UserListParams())
                }
                when (usersResult) {
                    is WpRequestResult.Success -> _users.value = usersResult.response.data
                    else -> _users.value = emptyList()
                }
            }
        }
    }
}
