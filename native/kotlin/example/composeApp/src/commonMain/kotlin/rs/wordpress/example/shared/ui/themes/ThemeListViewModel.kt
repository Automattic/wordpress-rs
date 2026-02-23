package rs.wordpress.example.shared.ui.themes

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import uniffi.wp_api.ThemeListParams
import uniffi.wp_api.ThemeWithEditContext

class ThemeListViewModel(private val apiClient: WpApiClient) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _themes = MutableStateFlow<List<ThemeWithEditContext>>(emptyList())
    val themes: StateFlow<List<ThemeWithEditContext>> = _themes.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    init {
        loadThemes()
    }

    private fun loadThemes() {
        viewModelScope.launch(Dispatchers.IO) {
            val result = apiClient.request { requestBuilder ->
                requestBuilder.themes().listWithEditContext(params = ThemeListParams())
            }
            when (result) {
                is WpRequestResult.Success -> _themes.value = result.response.data
                else -> _themes.value = emptyList()
            }
            _isLoading.value = false
        }
    }
}
