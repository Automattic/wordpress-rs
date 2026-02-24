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
import rs.wordpress.example.shared.ui.components.errorDescription
import uniffi.wp_api.PostTypeSupports
import uniffi.wp_api.TaxonomyListParams

data class SitePostType(
    val name: String,
    val restBase: String
)

data class SiteTaxonomy(
    val name: String,
    val restBase: String
)

class SiteViewModel(private val apiClient: WpApiClient) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _postTypes = MutableStateFlow<List<SitePostType>>(emptyList())
    val postTypes: StateFlow<List<SitePostType>> = _postTypes.asStateFlow()

    private val _taxonomies = MutableStateFlow<List<SiteTaxonomy>>(emptyList())
    val taxonomies: StateFlow<List<SiteTaxonomy>> = _taxonomies.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private var postTypesLoaded = false
    private var taxonomiesLoaded = false

    init {
        loadPostTypes()
        loadTaxonomies()
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
                else -> {
                    if (_error.value == null) _error.value = result.errorDescription()
                    _postTypes.value = emptyList()
                }
            }
            postTypesLoaded = true
            if (taxonomiesLoaded) _isLoading.value = false
        }
    }

    private fun loadTaxonomies() {
        viewModelScope.launch(Dispatchers.IO) {
            val result = apiClient.request { requestBuilder ->
                requestBuilder.taxonomies().listWithEditContext(params = TaxonomyListParams())
            }
            when (result) {
                is WpRequestResult.Success -> {
                    _taxonomies.value = result.response.data.taxonomyTypes
                        .values
                        .filter { it.visibility?.showInNavMenus == true }
                        .map { SiteTaxonomy(name = it.name, restBase = it.restBase) }
                }
                else -> {
                    if (_error.value == null) _error.value = result.errorDescription()
                    _taxonomies.value = emptyList()
                }
            }
            taxonomiesLoaded = true
            if (postTypesLoaded) _isLoading.value = false
        }
    }
}
