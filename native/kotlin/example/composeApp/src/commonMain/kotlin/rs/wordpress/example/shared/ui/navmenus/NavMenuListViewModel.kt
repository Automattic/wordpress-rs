package rs.wordpress.example.shared.ui.navmenus

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import rs.wordpress.example.shared.ui.components.errorDescription
import uniffi.wp_api.NavMenuListParams
import uniffi.wp_api.NavMenuWithEditContext

class NavMenuListViewModel(private val apiClient: WpApiClient) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _navMenus = MutableStateFlow<List<NavMenuWithEditContext>>(emptyList())
    val navMenus: StateFlow<List<NavMenuWithEditContext>> = _navMenus.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    init {
        loadNavMenus()
    }

    private fun loadNavMenus() {
        viewModelScope.launch(Dispatchers.IO) {
            val result = apiClient.request { requestBuilder ->
                requestBuilder.navMenus().listWithEditContext(NavMenuListParams())
            }
            when (result) {
                is WpRequestResult.Success -> _navMenus.value = result.response.data
                else -> {
                    _error.value = result.errorDescription()
                    _navMenus.value = emptyList()
                }
            }
            _isLoading.value = false
        }
    }
}
