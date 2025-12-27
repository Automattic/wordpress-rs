package rs.wordpress.example.shared.ui.postmetadatacollection

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.cache.kotlin.ObservableMetadataCollection
import rs.wordpress.cache.kotlin.getObservablePostMetadataCollectionWithEditContext
import rs.wordpress.cache.kotlin.hasMorePages
import rs.wordpress.cache.kotlin.isSyncing
import uniffi.wp_api.PostEndpointType
import uniffi.wp_mobile.ListInfo
import uniffi.wp_mobile.PostItemState
import uniffi.wp_mobile.PostListFilter
import uniffi.wp_mobile.PostMetadataCollectionItem
import uniffi.wp_mobile.SyncResult
import uniffi.wp_mobile.WpSelfHostedService
import uniffi.wp_mobile_cache.ListState

/**
 * UI state for the post metadata collection screen
 */
data class PostMetadataCollectionState(
    val currentFilter: PostListFilter,
    val listInfo: ListInfo? = null,
    val lastSyncResult: SyncResult? = null,
    val lastError: String? = null
) {
    /**
     * Whether a sync operation is in progress.
     * Derived from listInfo.state - the single source of truth from the database.
     */
    val isSyncing: Boolean
        get() = listInfo?.isSyncing ?: false

    val hasMorePages: Boolean
        get() = listInfo?.hasMorePages ?: true

    val currentPage: UInt?
        get() = listInfo?.currentPage

    val totalPages: UInt?
        get() = listInfo?.totalPages

    val syncState: ListState
        get() = listInfo?.state ?: ListState.IDLE

    val filterDisplayName: String
        get() {
            val statuses = currentFilter.status
            return when {
                statuses.isEmpty() -> "All Posts"
                statuses.any { it.toString().contains("draft", ignoreCase = true) } -> "Drafts"
                statuses.any { it.toString().contains("publish", ignoreCase = true) } -> "Published"
                else -> statuses.firstOrNull()?.toString() ?: "All Posts"
            }
        }

    val filterStatusString: String?
        get() = currentFilter.status.firstOrNull()?.toString()?.lowercase()
}

/**
 * Display data for a post item with its fetch state
 */
data class PostItemDisplayData(
    val id: Long,
    val state: PostItemState,
    val title: String?,
    val contentPreview: String?,
    val status: String?,
    val isLoading: Boolean,
    val errorMessage: String?
) {
    companion object {
        fun fromCollectionItem(item: PostMetadataCollectionItem): PostItemDisplayData {
            // Extract data from state variants that carry data
            val data = when (val s = item.state) {
                is PostItemState.Cached -> s.data
                is PostItemState.Stale -> s.data
                is PostItemState.FetchingWithData -> s.data
                is PostItemState.FailedWithData -> s.data
                else -> null
            }

            val isLoading = item.state is PostItemState.Fetching ||
                item.state is PostItemState.FetchingWithData

            val errorMessage = when (val s = item.state) {
                is PostItemState.Failed -> s.error
                is PostItemState.FailedWithData -> s.error
                else -> null
            }

            return PostItemDisplayData(
                id = item.id,
                state = item.state,
                title = data?.data?.title?.rendered,
                contentPreview = data?.data?.content?.rendered?.take(100),
                status = data?.data?.status?.toString(),
                isLoading = isLoading,
                errorMessage = errorMessage
            )
        }
    }
}

class PostMetadataCollectionViewModel(
    private val selfHostedService: WpSelfHostedService
) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _state = MutableStateFlow(PostMetadataCollectionState(currentFilter = PostListFilter()))
    val state: StateFlow<PostMetadataCollectionState> = _state.asStateFlow()

    private val _items = MutableStateFlow<List<PostItemDisplayData>>(emptyList())
    val items: StateFlow<List<PostItemDisplayData>> = _items.asStateFlow()

    private var observableCollection: ObservableMetadataCollection? = null

    init {
        createObservableCollection(_state.value.currentFilter)
        loadItemsFromCollection()
    }

    /**
     * Change the filter and load persisted state from database
     */
    fun setFilter(status: String?) {
        val postStatus = status?.let { uniffi.wp_api.parsePostStatus(it) }
        val newFilter = PostListFilter(
            status = if (postStatus != null) listOf(postStatus) else emptyList()
        )

        observableCollection?.close()
        createObservableCollection(newFilter)

        // Read persisted state from database (single query)
        _state.value = PostMetadataCollectionState(
            currentFilter = newFilter,
            listInfo = observableCollection?.listInfo(),
            lastSyncResult = null,
            lastError = null
        )

        // Load items (async)
        loadItemsFromCollection()
    }

    /**
     * Refresh the collection (fetch page 1, sync missing/stale)
     *
     * Note: syncState is managed by the database and observed via state observer.
     * We don't manually toggle isSyncing - it's derived from listInfo.state.
     */
    fun refresh() {
        if (_state.value.isSyncing) return

        _state.value = _state.value.copy(lastError = null)

        viewModelScope.launch(Dispatchers.IO) {
            try {
                val collection = observableCollection ?: return@launch
                val result = collection.refresh()

                _state.value = _state.value.copy(
                    listInfo = collection.listInfo(),
                    lastSyncResult = result,
                    lastError = null
                )
            } catch (e: Exception) {
                _state.value = _state.value.copy(
                    lastError = e.message ?: "Unknown error"
                )
            }
        }
    }

    /**
     * Load the next page of items
     *
     * Note: syncState is managed by the database and observed via state observer.
     * We don't manually toggle isSyncing - it's derived from listInfo.state.
     */
    fun loadNextPage() {
        if (_state.value.isSyncing) return
        if (!_state.value.hasMorePages) return

        // If no pages have been loaded yet, do a refresh instead
        if (_state.value.currentPage == null) {
            refresh()
            return
        }

        _state.value = _state.value.copy(lastError = null)

        viewModelScope.launch(Dispatchers.IO) {
            try {
                val collection = observableCollection ?: return@launch
                val result = collection.loadNextPage()

                _state.value = _state.value.copy(
                    listInfo = collection.listInfo(),
                    lastSyncResult = result,
                    lastError = null
                )
            } catch (e: Exception) {
                _state.value = _state.value.copy(
                    lastError = e.message ?: "Unknown error"
                )
            }
        }
    }

    private fun createObservableCollection(filter: PostListFilter) {
        val postService = selfHostedService.posts()
        val observable = postService.getObservablePostMetadataCollectionWithEditContext(
            PostEndpointType.Posts,
            filter
        )

        // Data observer: refresh list contents when data changes
        observable.addDataObserver {
            viewModelScope.launch(Dispatchers.Default) {
                loadItemsFromCollectionInternal()
            }
        }

        // ListInfo observer: update listInfo when pagination or sync state changes
        observable.addListInfoObserver {
            viewModelScope.launch(Dispatchers.Default) {
                updateListInfo()
            }
        }

        observableCollection = observable
    }

    private fun updateListInfo() {
        val newListInfo = observableCollection?.listInfo()
        println("[ViewModel] updateListInfo: new state = ${newListInfo?.state}")
        _state.value = _state.value.copy(listInfo = newListInfo)
    }

    private suspend fun loadItemsFromCollectionInternal() {
        try {
            val collection = observableCollection ?: return
            val rawItems = collection.loadItems()
            _items.value = rawItems.map { PostItemDisplayData.fromCollectionItem(it) }
        } catch (e: Exception) {
            println("Error loading items from collection: ${e.message}")
            _items.value = emptyList()
        }
    }

    private fun loadItemsFromCollection() {
        viewModelScope.launch(Dispatchers.Default) {
            loadItemsFromCollectionInternal()
        }
    }

    fun onCleared() {
        observableCollection?.close()
        observableCollection = null
        viewModelScope.cancel()
    }
}
