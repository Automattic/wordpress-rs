package rs.wordpress.example.shared.ui.terms

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import rs.wordpress.example.shared.ui.components.errorDescription
import uniffi.wp_api.AnyTermWithEditContext
import uniffi.wp_api.TermEndpointType
import uniffi.wp_api.TermListParams

class TermListByTypeViewModel(
    private val apiClient: WpApiClient,
    private val termEndpointType: TermEndpointType
) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _terms = MutableStateFlow<List<AnyTermWithEditContext>>(emptyList())
    val terms: StateFlow<List<AnyTermWithEditContext>> = _terms.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _isLoadingMore = MutableStateFlow(false)
    val isLoadingMore: StateFlow<Boolean> = _isLoadingMore.asStateFlow()

    private val _canLoadMore = MutableStateFlow(false)
    val canLoadMore: StateFlow<Boolean> = _canLoadMore.asStateFlow()

    private var nextPageParams: TermListParams? = null

    init {
        loadTerms()
    }

    private fun loadTerms() {
        viewModelScope.launch(Dispatchers.IO) {
            val result = apiClient.request { requestBuilder ->
                requestBuilder.terms().listWithEditContext(
                    termEndpointType = termEndpointType,
                    params = TermListParams()
                )
            }
            when (result) {
                is WpRequestResult.Success -> {
                    _terms.value = result.response.data
                    nextPageParams = result.response.nextPageParams
                    _canLoadMore.value = nextPageParams != null
                }
                else -> {
                    _error.value = result.errorDescription()
                    _terms.value = emptyList()
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
                requestBuilder.terms().listWithEditContext(
                    termEndpointType = termEndpointType,
                    params = params
                )
            }
            when (result) {
                is WpRequestResult.Success -> {
                    _terms.value = _terms.value + result.response.data
                    nextPageParams = result.response.nextPageParams
                    _canLoadMore.value = nextPageParams != null
                }
                else -> _error.value = result.errorDescription()
            }
            _isLoadingMore.value = false
        }
    }
}
