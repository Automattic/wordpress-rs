package rs.wordpress.example.shared.ui.search

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
import uniffi.wp_api.SearchListParams
import uniffi.wp_api.SearchResultWithViewContext

class SearchViewModel(private val apiClient: WpApiClient) : ViewModel() {

    private val _results = MutableStateFlow<List<SearchResultWithViewContext>>(emptyList())
    val results: StateFlow<List<SearchResultWithViewContext>> = _results.asStateFlow()

    private val _isLoading = MutableStateFlow(false)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _isLoadingMore = MutableStateFlow(false)
    val isLoadingMore: StateFlow<Boolean> = _isLoadingMore.asStateFlow()

    private val _canLoadMore = MutableStateFlow(false)
    val canLoadMore: StateFlow<Boolean> = _canLoadMore.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private val _hasSearched = MutableStateFlow(false)
    val hasSearched: StateFlow<Boolean> = _hasSearched.asStateFlow()

    private var nextPageParams: SearchListParams? = null

    fun search(query: String) {
        if (query.isBlank()) {
            _results.value = emptyList()
            nextPageParams = null
            _canLoadMore.value = false
            return
        }
        _error.value = null
        _hasSearched.value = true
        _isLoading.value = true
        nextPageParams = null
        _canLoadMore.value = false
        viewModelScope.launch(Dispatchers.IO) {
            val result = apiClient.request { requestBuilder ->
                requestBuilder.search().listWithViewContext(params = SearchListParams(search = query))
            }
            when (result) {
                is WpRequestResult.Success -> {
                    _results.value = result.response.data
                    nextPageParams = result.response.nextPageParams
                    _canLoadMore.value = nextPageParams != null
                }
                else -> {
                    _error.value = result.errorDescription()
                    _results.value = emptyList()
                }
            }
            _isLoading.value = false
        }
    }

    fun loadMore() {
        val params = nextPageParams ?: return
        if (_isLoadingMore.value) return
        _isLoadingMore.value = true
        viewModelScope.launch(Dispatchers.IO) {
            val result = apiClient.request { requestBuilder ->
                requestBuilder.search().listWithViewContext(params = params)
            }
            when (result) {
                is WpRequestResult.Success -> {
                    _results.value = _results.value + result.response.data
                    nextPageParams = result.response.nextPageParams
                    _canLoadMore.value = nextPageParams != null
                }
                else -> _error.value = result.errorDescription()
            }
            _isLoadingMore.value = false
        }
    }
}
