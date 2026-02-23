package rs.wordpress.example.shared.ui.search

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import uniffi.wp_api.SearchListParams
import uniffi.wp_api.SearchResultWithViewContext

class SearchViewModel(private val apiClient: WpApiClient) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _results = MutableStateFlow<List<SearchResultWithViewContext>>(emptyList())
    val results: StateFlow<List<SearchResultWithViewContext>> = _results.asStateFlow()

    private val _isLoading = MutableStateFlow(false)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    fun search(query: String) {
        if (query.isBlank()) {
            _results.value = emptyList()
            return
        }
        _isLoading.value = true
        viewModelScope.launch(Dispatchers.IO) {
            val result = apiClient.request { requestBuilder ->
                requestBuilder.search().listWithViewContext(params = SearchListParams(search = query))
            }
            when (result) {
                is WpRequestResult.Success -> _results.value = result.response.data
                else -> _results.value = emptyList()
            }
            _isLoading.value = false
        }
    }
}
