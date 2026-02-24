package rs.wordpress.example.shared.ui.settings

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
import uniffi.wp_api.SiteSettingsWithEditContext

class SiteSettingsViewModel(private val apiClient: WpApiClient) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _settings = MutableStateFlow<SiteSettingsWithEditContext?>(null)
    val settings: StateFlow<SiteSettingsWithEditContext?> = _settings.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    init {
        loadSettings()
    }

    private fun loadSettings() {
        viewModelScope.launch(Dispatchers.IO) {
            val result = apiClient.request { requestBuilder ->
                requestBuilder.siteSettings().retrieveWithEditContext()
            }
            when (result) {
                is WpRequestResult.Success -> _settings.value = result.response.data
                else -> {
                    _error.value = result.errorDescription()
                    _settings.value = null
                }
            }
            _isLoading.value = false
        }
    }
}
