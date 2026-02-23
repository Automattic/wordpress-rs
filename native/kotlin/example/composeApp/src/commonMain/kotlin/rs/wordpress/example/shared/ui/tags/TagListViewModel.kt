package rs.wordpress.example.shared.ui.tags

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

class TagListViewModel(private val apiClient: WpApiClient) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _tags = MutableStateFlow<List<AnyTermWithEditContext>>(emptyList())
    val tags: StateFlow<List<AnyTermWithEditContext>> = _tags.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _isLoadingMore = MutableStateFlow(false)
    val isLoadingMore: StateFlow<Boolean> = _isLoadingMore.asStateFlow()

    private val _canLoadMore = MutableStateFlow(false)
    val canLoadMore: StateFlow<Boolean> = _canLoadMore.asStateFlow()

    private var nextPageParams: TermListParams? = null

    init {
        loadTags()
    }

    private fun loadTags() {
        viewModelScope.launch(Dispatchers.IO) {
            val result = apiClient.request { requestBuilder ->
                requestBuilder.terms().listWithEditContext(
                    termEndpointType = TermEndpointType.Tags,
                    params = TermListParams()
                )
            }
            when (result) {
                is WpRequestResult.Success -> {
                    _tags.value = result.response.data
                    nextPageParams = result.response.nextPageParams
                    _canLoadMore.value = nextPageParams != null
                }
                else -> _tags.value = emptyList()
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
                requestBuilder.terms().listWithEditContext(
                    termEndpointType = TermEndpointType.Tags,
                    params = params
                )
            }
            when (result) {
                is WpRequestResult.Success -> {
                    _tags.value = _tags.value + result.response.data
                    nextPageParams = result.response.nextPageParams
                    _canLoadMore.value = nextPageParams != null
                }
                else -> {}
            }
            _isLoadingMore.value = false
        }
    }
}
