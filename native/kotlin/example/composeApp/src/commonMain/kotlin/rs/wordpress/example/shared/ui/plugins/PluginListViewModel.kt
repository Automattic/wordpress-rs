package rs.wordpress.example.shared.ui.plugins

import androidx.lifecycle.ViewModel
import kotlinx.coroutines.Dispatchers
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import rs.wordpress.example.shared.ui.components.errorDescription
import uniffi.wp_api.PluginListParams
import uniffi.wp_api.PluginWithEditContext

class PluginListViewModel(private val apiClient: WpApiClient) : ViewModel() {

    private val _plugins = MutableStateFlow<List<PluginWithEditContext>>(emptyList())
    val plugins: StateFlow<List<PluginWithEditContext>> = _plugins.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    init {
        loadPlugins()
    }

    private fun loadPlugins() {
        viewModelScope.launch(Dispatchers.IO) {
            val pluginsResult = apiClient.request { requestBuilder ->
                requestBuilder.plugins().listWithEditContext(params = PluginListParams())
            }
            when (pluginsResult) {
                is WpRequestResult.Success -> _plugins.value = pluginsResult.response.data
                else -> {
                    _error.value = pluginsResult.errorDescription()
                    _plugins.value = emptyList()
                }
            }
            _isLoading.value = false
        }
    }
}
