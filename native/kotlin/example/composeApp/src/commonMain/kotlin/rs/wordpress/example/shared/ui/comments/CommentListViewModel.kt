package rs.wordpress.example.shared.ui.comments

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import uniffi.wp_api.CommentListParams
import uniffi.wp_api.CommentWithEditContext

class CommentListViewModel(private val apiClient: WpApiClient) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _comments = MutableStateFlow<List<CommentWithEditContext>>(emptyList())
    val comments: StateFlow<List<CommentWithEditContext>> = _comments.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    init {
        loadComments()
    }

    private fun loadComments() {
        viewModelScope.launch(Dispatchers.IO) {
            val result = apiClient.request { requestBuilder ->
                requestBuilder.comments().listWithEditContext(params = CommentListParams())
            }
            when (result) {
                is WpRequestResult.Success -> _comments.value = result.response.data
                else -> _comments.value = emptyList()
            }
            _isLoading.value = false
        }
    }
}
