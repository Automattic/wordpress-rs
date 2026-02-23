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

    private val _isLoadingMore = MutableStateFlow(false)
    val isLoadingMore: StateFlow<Boolean> = _isLoadingMore.asStateFlow()

    private val _canLoadMore = MutableStateFlow(false)
    val canLoadMore: StateFlow<Boolean> = _canLoadMore.asStateFlow()

    private var nextPageParams: SearchListParams? = null

    fun search(query: String) {
        if (query.isBlank()) {
            _results.value = emptyList()
            nextPageParams = null
            _canLoadMore.value = false
            return
        }
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
                else -> _results.value = emptyList()
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
                else -> {}
            }
            _isLoadingMore.value = false
        }
    }
}
