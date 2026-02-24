package rs.wordpress.example.shared.ui.menulocations

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import uniffi.wp_api.MenuLocationWithEditContext

class MenuLocationListViewModel(private val apiClient: WpApiClient) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _menuLocations = MutableStateFlow<List<MenuLocationWithEditContext>>(emptyList())
    val menuLocations: StateFlow<List<MenuLocationWithEditContext>> = _menuLocations.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    init {
        loadMenuLocations()
    }

    private fun loadMenuLocations() {
        viewModelScope.launch(Dispatchers.IO) {
            val result = apiClient.request { requestBuilder ->
                requestBuilder.menuLocations().listWithEditContext()
            }
            when (result) {
                is WpRequestResult.Success -> {
                    _menuLocations.value = result.response.data.locations.values.toList()
                }
                else -> _menuLocations.value = emptyList()
            }
            _isLoading.value = false
        }
    }
}
