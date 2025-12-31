package rs.wordpress.example.shared.ui.posttypes

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.cache.kotlin.ObservableCollection
import rs.wordpress.cache.kotlin.getObservablePostTypeCollectionWithEditContext
import uniffi.wp_mobile.FullEntityPostTypeDetailsWithEditContext
import uniffi.wp_mobile.PostTypeCollectionWithEditContext
import uniffi.wp_mobile.WpSelfHostedService

/**
 * UI state for the post types screen
 */
data class PostTypesState(
    val isFetching: Boolean = false,
    val lastError: String? = null,
    val hasFetchedOnce: Boolean = false
)

/**
 * Display data for a post type
 */
data class PostTypeDisplayData(
    val slug: String,
    val name: String,
    val description: String?,
    val hierarchical: Boolean,
    val restBase: String?
) {
    companion object {
        fun fromEntity(entity: FullEntityPostTypeDetailsWithEditContext): PostTypeDisplayData {
            val postType = entity.data
            return PostTypeDisplayData(
                slug = postType.slug,
                name = postType.name,
                description = postType.description,
                hierarchical = postType.hierarchical ?: false,
                restBase = postType.restBase
            )
        }
    }
}

class PostTypesViewModel(
    private val selfHostedService: WpSelfHostedService
) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _state = MutableStateFlow(PostTypesState())
    val state: StateFlow<PostTypesState> = _state.asStateFlow()

    private val _postTypes = MutableStateFlow<List<PostTypeDisplayData>>(emptyList())
    val postTypes: StateFlow<List<PostTypeDisplayData>> = _postTypes.asStateFlow()

    private var observableCollection: ObservableCollection<FullEntityPostTypeDetailsWithEditContext>? = null
    private var postTypeCollection: PostTypeCollectionWithEditContext? = null

    init {
        createObservableCollection()
        loadPostTypesFromCache()
        // Auto-fetch on init to ensure we have post types
        fetch()
    }

    /**
     * Fetch all post types from the network
     */
    fun fetch() {
        if (_state.value.isFetching) {
            return // Already fetching, ignore
        }

        _state.value = _state.value.copy(
            isFetching = true,
            lastError = null
        )

        viewModelScope.launch(Dispatchers.IO) {
            try {
                val collection = postTypeCollection
                if (collection == null) {
                    _state.value = _state.value.copy(isFetching = false)
                    return@launch
                }

                // Fetch all post types (no pagination)
                collection.fetch()

                // Update state with successful result
                _state.value = _state.value.copy(
                    isFetching = false,
                    lastError = null,
                    hasFetchedOnce = true
                )

                // Post types will auto-reload via ObservableCollection after database update
            } catch (error: Exception) {
                // Update state with error
                _state.value = _state.value.copy(
                    lastError = error.message ?: "Unknown error",
                    isFetching = false
                )
            }
        }
    }

    /**
     * Create the observable collection
     */
    private fun createObservableCollection() {
        val postTypeService = selfHostedService.postTypes()

        // Create the underlying PostTypeCollection (for fetch)
        // Uses default filter (viewable = true)
        val underlyingCollection = postTypeService.createPostTypeCollectionWithEditContext()
        postTypeCollection = underlyingCollection

        // Create observable wrapper (for auto-reload on DB changes)
        val observable = postTypeService.getObservablePostTypeCollectionWithEditContext()

        // Set up observer to reload post types when database changes
        observable.addObserver {
            loadPostTypesFromCache()
        }

        observableCollection = observable
    }

    /**
     * Load post types from cache and update the state flow
     */
    private fun loadPostTypesFromCache() {
        viewModelScope.launch(Dispatchers.Default) {
            try {
                val collection = observableCollection ?: return@launch
                val allPostTypes = collection.loadData()

                val postTypeDataList = allPostTypes.map { fullEntity ->
                    PostTypeDisplayData.fromEntity(fullEntity)
                }

                _postTypes.value = postTypeDataList
            } catch (e: Exception) {
                println("Error loading post types from cache: ${e.message}")
                _postTypes.value = emptyList()
            }
        }
    }

    /**
     * Clean up resources when ViewModel is destroyed
     */
    fun onCleared() {
        observableCollection?.close()
        observableCollection = null
        viewModelScope.cancel()
    }
}
