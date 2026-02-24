package rs.wordpress.example.shared.ui.applicationpasswords

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import uniffi.wp_api.ApplicationPasswordWithEditContext

class ApplicationPasswordListViewModel(private val apiClient: WpApiClient) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _applicationPasswords = MutableStateFlow<List<ApplicationPasswordWithEditContext>>(emptyList())
    val applicationPasswords: StateFlow<List<ApplicationPasswordWithEditContext>> = _applicationPasswords.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    init {
        loadApplicationPasswords()
    }

    private fun loadApplicationPasswords() {
        viewModelScope.launch(Dispatchers.IO) {
            val meResult = apiClient.request { requestBuilder ->
                requestBuilder.users().retrieveMeWithEditContext()
            }
            val userId = when (meResult) {
                is WpRequestResult.Success -> meResult.response.data.id
                else -> {
                    _isLoading.value = false
                    return@launch
                }
            }

            val result = apiClient.request { requestBuilder ->
                requestBuilder.applicationPasswords().listWithEditContext(userId)
            }
            when (result) {
                is WpRequestResult.Success -> _applicationPasswords.value = result.response.data
                else -> _applicationPasswords.value = emptyList()
            }
            _isLoading.value = false
        }
    }
}
