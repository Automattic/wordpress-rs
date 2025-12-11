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
import uniffi.wp_mobile.AnyPostFilter
import uniffi.wp_mobile.EntityState
import uniffi.wp_mobile.PostMetadataCollectionItem
import uniffi.wp_mobile.SyncResult
import uniffi.wp_mobile.WpSelfHostedService
import uniffi.wp_mobile_cache.ListState

/**
 * UI state for the post metadata collection screen
 */
data class PostMetadataCollectionState(
    val currentFilter: AnyPostFilter,
    val currentPage: UInt = 0u,
    val totalPages: UInt? = null,
    val lastSyncResult: SyncResult? = null,
    val lastError: String? = null,
    val isSyncing: Boolean = false,
    val syncState: ListState = ListState.IDLE
) {
    val hasMorePages: Boolean
        get() = totalPages?.let { currentPage < it } ?: true

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

    val filterStatusString: String?
        get() = currentFilter.status?.toString()?.lowercase()
}

/**
 * Display data for a post item with its fetch state
 */
data class PostItemDisplayData(
    val id: Long,
    val state: EntityState,
    val title: String?,
    val contentPreview: String?,
    val status: String?,
    val isLoading: Boolean,
    val errorMessage: String?
) {
    companion object {
        fun fromCollectionItem(item: PostMetadataCollectionItem): PostItemDisplayData {
            val data = item.data
            return PostItemDisplayData(
                id = item.id,
                state = item.state,
                title = data?.data?.title?.rendered,
                contentPreview = data?.data?.content?.rendered?.take(100),
                status = data?.data?.status?.toString(),
                isLoading = item.state is EntityState.Fetching,
                errorMessage = when (val s = item.state) {
                    is EntityState.Failed -> s.error
                    else -> null
                }
            )
        }
    }
}

class PostMetadataCollectionViewModel(
    private val selfHostedService: WpSelfHostedService
) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _state = MutableStateFlow(PostMetadataCollectionState(currentFilter = AnyPostFilter(null)))
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
        val newFilter = AnyPostFilter(status = postStatus)

        observableCollection?.close()
        createObservableCollection(newFilter)

        // Read persisted pagination state from database (sync values)
        val collection = observableCollection
        _state.value = PostMetadataCollectionState(
            currentFilter = newFilter,
            currentPage = collection?.currentPage() ?: 0u,
            totalPages = collection?.totalPages(),
            lastSyncResult = null,
            lastError = null,
            isSyncing = false,
            syncState = ListState.IDLE
        )

        // Load items and syncState (async)
        viewModelScope.launch(Dispatchers.Default) {
            loadItemsFromCollectionInternal()
            updateSyncState()
        }
    }

    /**
     * Refresh the collection (fetch page 1, sync missing/stale)
     */
    fun refresh() {
        if (_state.value.isSyncing) return

        _state.value = _state.value.copy(isSyncing = true, lastError = null)

        viewModelScope.launch(Dispatchers.IO) {
            try {
                val collection = observableCollection ?: return@launch
                val result = collection.refresh()

                _state.value = _state.value.copy(
                    currentPage = collection.currentPage(),
                    totalPages = collection.totalPages(),
                    lastSyncResult = result,
                    lastError = null,
                    isSyncing = false,
                    syncState = collection.syncState()
                )

                loadItemsFromCollection()
            } catch (e: Exception) {
                _state.value = _state.value.copy(
                    lastError = e.message ?: "Unknown error",
                    isSyncing = false,
                    syncState = observableCollection?.syncState() ?: _state.value.syncState
                )
            }
        }
    }

    /**
     * Load the next page of items
     */
    fun loadNextPage() {
        if (_state.value.isSyncing) return
        if (!_state.value.hasMorePages) return

        // If no pages have been loaded yet, do a refresh instead
        if (_state.value.currentPage == 0u) {
            refresh()
            return
        }

        _state.value = _state.value.copy(isSyncing = true, lastError = null)

        viewModelScope.launch(Dispatchers.IO) {
            try {
                val collection = observableCollection ?: return@launch
                val result = collection.loadNextPage()

                _state.value = _state.value.copy(
                    currentPage = collection.currentPage(),
                    totalPages = collection.totalPages(),
                    lastSyncResult = result,
                    lastError = null,
                    isSyncing = false,
                    syncState = collection.syncState()
                )

                loadItemsFromCollection()
            } catch (e: Exception) {
                _state.value = _state.value.copy(
                    lastError = e.message ?: "Unknown error",
                    isSyncing = false,
                    syncState = observableCollection?.syncState() ?: _state.value.syncState
                )
            }
        }
    }

    private fun createObservableCollection(filter: AnyPostFilter) {
        val postService = selfHostedService.posts()
        val observable = postService.getObservablePostMetadataCollectionWithEditContext(filter)

        // Data observer: refresh list contents when data changes
        // Note: Must dispatch to coroutine since loadItems() is a suspend function
        observable.addDataObserver {
            viewModelScope.launch(Dispatchers.Default) {
                loadItemsFromCollectionInternal()
            }
        }

        // State observer: update sync state indicator when state changes
        // Note: Must dispatch to coroutine since syncState() is a suspend function
        observable.addStateObserver {
            viewModelScope.launch(Dispatchers.Default) {
                updateSyncState()
            }
        }

        observableCollection = observable
    }

    private suspend fun updateSyncState() {
        val collection = observableCollection ?: return
        val newSyncState = collection.syncState()
        println("[ViewModel] updateSyncState: new state = $newSyncState")
        _state.value = _state.value.copy(syncState = newSyncState)
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
