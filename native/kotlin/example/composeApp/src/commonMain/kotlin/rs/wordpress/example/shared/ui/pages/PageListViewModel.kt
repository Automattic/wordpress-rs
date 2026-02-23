package rs.wordpress.example.shared.ui.pages

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import uniffi.wp_api.AnyPostWithEditContext
import uniffi.wp_api.PostEndpointType
import uniffi.wp_api.PostListParams

class PageListViewModel(private val apiClient: WpApiClient) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _pages = MutableStateFlow<List<AnyPostWithEditContext>>(emptyList())
    val pages: StateFlow<List<AnyPostWithEditContext>> = _pages.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _isLoadingMore = MutableStateFlow(false)
    val isLoadingMore: StateFlow<Boolean> = _isLoadingMore.asStateFlow()

    private val _canLoadMore = MutableStateFlow(false)
    val canLoadMore: StateFlow<Boolean> = _canLoadMore.asStateFlow()

    private var nextPageParams: PostListParams? = null

    init {
        loadPages()
    }

    private fun loadPages() {
        viewModelScope.launch(Dispatchers.IO) {
            val result = apiClient.request { requestBuilder ->
                requestBuilder.posts().listWithEditContext(
                    postEndpointType = PostEndpointType.Pages,
                    params = PostListParams()
                )
            }
            when (result) {
                is WpRequestResult.Success -> {
                    _pages.value = result.response.data
                    nextPageParams = result.response.nextPageParams
                    _canLoadMore.value = nextPageParams != null
                }
                else -> _pages.value = emptyList()
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
                requestBuilder.posts().listWithEditContext(
                    postEndpointType = PostEndpointType.Pages,
                    params = params
                )
            }
            when (result) {
                is WpRequestResult.Success -> {
                    _pages.value = _pages.value + result.response.data
                    nextPageParams = result.response.nextPageParams
                    _canLoadMore.value = nextPageParams != null
                }
                else -> {}
            }
            _isLoadingMore.value = false
        }
    }
}
