package rs.wordpress.example.shared.ui.categories

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import uniffi.wp_api.AnyTermWithEditContext
import uniffi.wp_api.TermEndpointType
import uniffi.wp_api.TermListParams

class CategoryListViewModel(private val apiClient: WpApiClient) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _categories = MutableStateFlow<List<AnyTermWithEditContext>>(emptyList())
    val categories: StateFlow<List<AnyTermWithEditContext>> = _categories.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    init {
        loadCategories()
    }

    private fun loadCategories() {
        viewModelScope.launch(Dispatchers.IO) {
            val result = apiClient.request { requestBuilder ->
                requestBuilder.terms().listWithEditContext(
                    termEndpointType = TermEndpointType.Categories,
                    params = TermListParams()
                )
            }
            when (result) {
                is WpRequestResult.Success -> _categories.value = result.response.data
                else -> _categories.value = emptyList()
            }
            _isLoading.value = false
        }
    }
}
