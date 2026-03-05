package rs.wordpress.example.shared.ui.navigations

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
import uniffi.wp_api.NavigationListParams
import uniffi.wp_api.NavigationWithEditContext

class NavigationListViewModel(private val apiClient: WpApiClient) : ViewModel() {

    private val _navigations = MutableStateFlow<List<NavigationWithEditContext>>(emptyList())
    val navigations: StateFlow<List<NavigationWithEditContext>> = _navigations.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _isLoadingMore = MutableStateFlow(false)
    val isLoadingMore: StateFlow<Boolean> = _isLoadingMore.asStateFlow()

    private val _canLoadMore = MutableStateFlow(false)
    val canLoadMore: StateFlow<Boolean> = _canLoadMore.asStateFlow()

    private var nextPageParams: NavigationListParams? = null

    init {
        loadNavigations()
    }

    private fun loadNavigations() {
        viewModelScope.launch(Dispatchers.IO) {
            val result = apiClient.request { requestBuilder ->
                requestBuilder.navigations().listWithEditContext(NavigationListParams())
            }
            when (result) {
                is WpRequestResult.Success -> {
                    _navigations.value = result.response.data
                    nextPageParams = result.response.nextPageParams
                    _canLoadMore.value = nextPageParams != null
                }
                else -> {
                    _error.value = result.errorDescription()
                    _navigations.value = emptyList()
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
                requestBuilder.navigations().listWithEditContext(params)
            }
            when (result) {
                is WpRequestResult.Success -> {
                    _navigations.value = _navigations.value + result.response.data
                    nextPageParams = result.response.nextPageParams
                    _canLoadMore.value = nextPageParams != null
                }
                else -> _error.value = result.errorDescription()
            }
            _isLoadingMore.value = false
        }
    }
}
