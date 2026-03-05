package rs.wordpress.example.shared.ui.posts

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
import uniffi.wp_api.AnyPostWithEditContext
import uniffi.wp_api.PostEndpointType
import uniffi.wp_api.PostListParams

class PostListByTypeViewModel(
    private val apiClient: WpApiClient,
    private val postEndpointType: PostEndpointType
) : ViewModel() {

    private val _posts = MutableStateFlow<List<AnyPostWithEditContext>>(emptyList())
    val posts: StateFlow<List<AnyPostWithEditContext>> = _posts.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _isLoadingMore = MutableStateFlow(false)
    val isLoadingMore: StateFlow<Boolean> = _isLoadingMore.asStateFlow()

    private val _canLoadMore = MutableStateFlow(false)
    val canLoadMore: StateFlow<Boolean> = _canLoadMore.asStateFlow()

    private var nextPageParams: PostListParams? = null

    init {
        loadPosts()
    }

    private fun loadPosts() {
        viewModelScope.launch(Dispatchers.IO) {
            val result = apiClient.request { requestBuilder ->
                requestBuilder.posts().listWithEditContext(
                    postEndpointType = postEndpointType,
                    params = PostListParams()
                )
            }
            when (result) {
                is WpRequestResult.Success -> {
                    _posts.value = result.response.data
                    nextPageParams = result.response.nextPageParams
                    _canLoadMore.value = nextPageParams != null
                }
                else -> {
                    _error.value = result.errorDescription()
                    _posts.value = emptyList()
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
                requestBuilder.posts().listWithEditContext(
                    postEndpointType = postEndpointType,
                    params = params
                )
            }
            when (result) {
                is WpRequestResult.Success -> {
                    _posts.value = _posts.value + result.response.data
                    nextPageParams = result.response.nextPageParams
                    _canLoadMore.value = nextPageParams != null
                }
                else -> _error.value = result.errorDescription()
            }
            _isLoadingMore.value = false
        }
    }
}
