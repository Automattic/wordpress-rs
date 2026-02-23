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

class UserListViewModel(private val apiClient: WpApiClient) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _users = MutableStateFlow<List<UserWithEditContext>>(emptyList())
    val users: StateFlow<List<UserWithEditContext>> = _users.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    init {
        loadUsers()
    }

    private fun loadUsers() {
        viewModelScope.launch(Dispatchers.IO) {
            val usersResult = apiClient.request { requestBuilder ->
                requestBuilder.users().listWithEditContext(params = UserListParams())
            }
            when (usersResult) {
                is WpRequestResult.Success -> _users.value = usersResult.response.data
                else -> _users.value = emptyList()
            }
            _isLoading.value = false
        }
    }
}
