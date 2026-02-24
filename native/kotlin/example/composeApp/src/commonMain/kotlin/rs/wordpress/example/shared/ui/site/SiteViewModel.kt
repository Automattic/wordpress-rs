package rs.wordpress.example.shared.ui.site

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import uniffi.wp_api.PostTypeSupports

data class SitePostType(
    val name: String,
    val restBase: String
)

class SiteViewModel(private val apiClient: WpApiClient) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _postTypes = MutableStateFlow<List<SitePostType>>(emptyList())
    val postTypes: StateFlow<List<SitePostType>> = _postTypes.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    init {
        loadPostTypes()
    }

    private fun loadPostTypes() {
        viewModelScope.launch(Dispatchers.IO) {
            val result = apiClient.request { requestBuilder ->
                requestBuilder.postTypes().listWithEditContext()
            }
            when (result) {
                is WpRequestResult.Success -> {
                    _postTypes.value = result.response.data.postTypes
                        .values
                        .filter { it.visibility?.showInNavMenus == true }
                        .filter { postType ->
                            postType.supports.supports(PostTypeSupports.Title) &&
                                postType.supports.supports(PostTypeSupports.Author) &&
                                postType.supports.supports(PostTypeSupports.CustomFields)
                        }
                        .map { SitePostType(name = it.name, restBase = it.restBase) }
                }
                else -> _postTypes.value = emptyList()
            }
            _isLoading.value = false
        }
    }
}
