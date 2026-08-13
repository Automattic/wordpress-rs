package rs.wordpress.example.shared.ui.postcollection

import androidx.lifecycle.ViewModel
import kotlinx.coroutines.Dispatchers
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.cache.kotlin.ObservableCollection
import rs.wordpress.cache.kotlin.getObservablePostCollectionWithEditContext
import rs.wordpress.example.shared.ui.stresstest.PostDisplayData
import uniffi.wp_mobile.AnyPostFilter
import uniffi.wp_mobile.FetchResult
import uniffi.wp_mobile.FullEntityAnyPostWithEditContext
import uniffi.wp_mobile.PostCollectionWithEditContext
import uniffi.wp_mobile.WpService

/**
 * UI state for the post collection screen
 */
data class CollectionState(
    val currentFilter: AnyPostFilter,
    val currentPage: Int = 0,
    val lastFetchResult: FetchResult? = null,
    val lastFetchError: Any? = null,
    val isFetching: Boolean = false
) : ViewModel() {
    val nextPage: Int get() = currentPage + 1

    /**
     * Returns the filter status as a lowercase string for matching in UI
     * null if no filter is set (All posts)
     */
    val filterStatusString: String?
        get() = currentFilter.status?.toString()?.lowercase()

    val filterDisplayName: String
        get() {
            val status = currentFilter.status
            val statusString = status?.toString() ?: ""
            return when {
                status == null -> "All Posts"
                statusString.contains("draft", ignoreCase = true) -> "Drafts"
                statusString.contains("publish", ignoreCase = true) -> "Published"
                else -> statusString
            }
        }
}

class PostCollectionViewModel(
    private val selfHostedService: WpService
) : ViewModel() {

    private val _state = MutableStateFlow(CollectionState(currentFilter = AnyPostFilter(null)))
    val state: StateFlow<CollectionState> = _state.asStateFlow()

    private val _posts = MutableStateFlow<List<PostDisplayData>>(emptyList())
    val posts: StateFlow<List<PostDisplayData>> = _posts.asStateFlow()

    private var observableCollection: ObservableCollection<FullEntityAnyPostWithEditContext>? = null
    private var postCollection: PostCollectionWithEditContext? = null

    companion object {
        private const val PER_PAGE = 20u
    }

    init {
        // Initialize with the default filter (All Posts)
        createObservableCollection(_state.value.currentFilter)
        loadPostsFromCache()
    }

    /**
     * Change the filter and reset pagination
     */
    fun setFilter(status: String?) {
        // Parse the string status to PostStatus using the helper function
        val postStatus = status?.let { uniffi.wp_api.postStatusFromString(it) }
        val newFilter = AnyPostFilter(status = postStatus)

        // Update state: new filter, reset page, clear fetch results
        _state.value = CollectionState(
            currentFilter = newFilter,
            currentPage = 0,
            lastFetchResult = null,
            lastFetchError = null,
            isFetching = false
        )

        // Close old observable before creating new one
        observableCollection?.close()

        // Create new observable collection with the new filter
        createObservableCollection(newFilter)

        // Load posts from cache (will show cached posts matching new filter)
        loadPostsFromCache()
    }

    /**
     * Fetch the next page from the network
     */
    fun fetchNextPage() {
        if (_state.value.isFetching) {
            return // Already fetching, ignore
        }

        _state.value = _state.value.copy(
            isFetching = true,
            lastFetchError = null
        )

        viewModelScope.launch(Dispatchers.IO) {
            try {
                val collection = postCollection
                if (collection == null) {
                    _state.value = _state.value.copy(isFetching = false)
                    return@launch
                }

                // Fetch the next page
                val result = collection.fetchPage(_state.value.nextPage.toUInt(), PER_PAGE)

                // Update state with successful result
                _state.value = _state.value.copy(
                    currentPage = _state.value.nextPage,
                    lastFetchResult = result,
                    lastFetchError = null,
                    isFetching = false
                )

                // Posts will auto-reload via ObservableCollection after database update
            } catch (error: Exception) {
                // Update state with error
                _state.value = _state.value.copy(
                    lastFetchError = error,
                    isFetching = false
                )
            }
        }
    }

    /**
     * Create a new observable collection with the given filter
     */
    private fun createObservableCollection(filter: AnyPostFilter) {
        val postService = selfHostedService.posts()

        // Create the underlying PostCollection (for fetchPage)
        val underlyingCollection = postService.createPostCollectionWithEditContext(filter)
        postCollection = underlyingCollection

        // Create observable wrapper (for auto-reload on DB changes)
        val observable = postService.getObservablePostCollectionWithEditContext(filter)

        // Set up observer to reload posts when database changes
        observable.addObserver {
            loadPostsFromCache()
        }

        observableCollection = observable
    }

    /**
     * Load posts from cache and update the posts state flow
     */
    private fun loadPostsFromCache() {
        viewModelScope.launch(Dispatchers.Default) {
            try {
                val collection = observableCollection ?: return@launch
                val allPosts = collection.loadData()

                val postDataList = allPosts.map { fullEntity ->
                    PostDisplayData(
                        entityId = fullEntity.entityId,
                        title = fullEntity.data.title?.rendered ?: "<no-title>",
                        contentPreview = fullEntity.data.content.rendered.take(100),
                        status = fullEntity.data.status.toString(),
                        date = fullEntity.data.date.value,
                        modified = fullEntity.data.modified.value,
                        author = fullEntity.data.author?.toString()
                    )
                }

                _posts.value = postDataList
            } catch (e: Exception) {
                println("Error loading posts from cache: ${e.message}")
                _posts.value = emptyList()
            }
        }
    }

    /**
     * Clean up resources when ViewModel is destroyed
     */
    override fun onCleared() {
        // Close the observable collection to unregister from DatabaseChangeNotifier
        observableCollection?.close()
        observableCollection = null
    }
}
