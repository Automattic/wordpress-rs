package rs.wordpress.example.shared.ui.comments

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
import uniffi.wp_api.CommentListParams
import uniffi.wp_api.CommentWithEditContext

class CommentListViewModel(private val apiClient: WpApiClient) : ViewModel() {

    private val _comments = MutableStateFlow<List<CommentWithEditContext>>(emptyList())
    val comments: StateFlow<List<CommentWithEditContext>> = _comments.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _isLoadingMore = MutableStateFlow(false)
    val isLoadingMore: StateFlow<Boolean> = _isLoadingMore.asStateFlow()

    private val _canLoadMore = MutableStateFlow(false)
    val canLoadMore: StateFlow<Boolean> = _canLoadMore.asStateFlow()

    private var nextPageParams: CommentListParams? = null

    init {
        loadComments()
    }

    private fun loadComments() {
        viewModelScope.launch(Dispatchers.IO) {
            val result = apiClient.request { requestBuilder ->
                requestBuilder.comments().listWithEditContext(params = CommentListParams())
            }
            when (result) {
                is WpRequestResult.Success -> {
                    _comments.value = result.response.data
                    nextPageParams = result.response.nextPageParams
                    _canLoadMore.value = nextPageParams != null
                }
                else -> {
                    _error.value = result.errorDescription()
                    _comments.value = emptyList()
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
                requestBuilder.comments().listWithEditContext(params = params)
            }
            when (result) {
                is WpRequestResult.Success -> {
                    _comments.value = _comments.value + result.response.data
                    nextPageParams = result.response.nextPageParams
                    _canLoadMore.value = nextPageParams != null
                }
                else -> _error.value = result.errorDescription()
            }
            _isLoadingMore.value = false
        }
    }
}
