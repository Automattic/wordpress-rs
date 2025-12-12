package rs.wordpress.cache.kotlin

import uniffi.wp_mobile.PostMetadataCollectionItem
import uniffi.wp_mobile.PostMetadataCollectionWithEditContext
import uniffi.wp_mobile.SyncResult
import uniffi.wp_mobile_cache.ListState
import uniffi.wp_mobile_cache.UpdateHook
import java.util.concurrent.CopyOnWriteArrayList

// Design note: State representation could be moved to Rust with proper enum modeling.
// See metadata_collection_v3.md for "Refined State Representation" design.
// Current design uses separate fields (id, state, data); could be a sealed class for type safety.
// The current EntityState enum doesn't carry data, so we assemble the full state in Kotlin.

/**
 * Create an observable metadata collection that notifies observers when data changes.
 *
 * This helper automatically registers the collection with [DatabaseChangeNotifier].
 * Use service extension functions (e.g., `getObservablePostMetadataCollectionWithEditContext`)
 * instead of calling this directly.
 *
 * **Lifecycle Management**: Collections implement [AutoCloseable] and should be closed when
 * no longer needed to prevent memory accumulation. In ViewModels, call `.close()` in
 * `onCleared()`. For short-lived usage, use `.use { }` blocks. For app-lifecycle-scoped
 * observables, explicit cleanup may not be necessary.
 *
 * Example (ViewModel):
 * ```
 * class MyViewModel : ViewModel() {
 *     private val observableCollection = postService.getObservablePostMetadataCollectionWithEditContext(filter)
 *
 *     init {
 *         observableCollection.addObserver { /* update UI */ }
 *         viewModelScope.launch { observableCollection.refresh() }
 *     }
 *
 *     override fun onCleared() {
 *         super.onCleared()
 *         observableCollection.close()
 *     }
 * }
 * ```
 */
fun createObservableMetadataCollection(
    collection: PostMetadataCollectionWithEditContext
): ObservableMetadataCollection = ObservableMetadataCollection(
    collection = collection
).also {
    DatabaseChangeNotifier.register(it)
}

/**
 * Observable wrapper around a metadata collection that notifies observers when changes occur.
 *
 * This is similar to [ObservableCollection] but designed for the "metadata-first" sync strategy:
 * - Items include fetch state (Missing, Fetching, Cached, Stale, Failed)
 * - Sync operations (refresh, loadNextPage) are exposed for explicit control
 * - Data is optional per item (present only when Cached)
 *
 * The metadata collection uses a two-phase sync:
 * 1. Fetch lightweight metadata (id + modified_gmt) to define list structure
 * 2. Selectively fetch full data for missing or stale items
 *
 * This allows showing cached items immediately while loading only what's needed.
 *
 * Create instances using [createObservableMetadataCollection] or service extension functions
 * rather than the constructor directly.
 *
 * Implements [AutoCloseable] to support cleanup. Call [close] when done (typically in
 * ViewModel.onCleared()) to unregister from [DatabaseChangeNotifier].
 */
@Suppress("TooManyFunctions") // Observer pattern requires multiple add/remove/notify methods
class ObservableMetadataCollection(
    private val collection: PostMetadataCollectionWithEditContext
) : AutoCloseable {
    private val dataObservers = CopyOnWriteArrayList<() -> Unit>()
    private val stateObservers = CopyOnWriteArrayList<() -> Unit>()

    /**
     * Add an observer for data changes (list contents changed).
     *
     * Data observers are notified when:
     * - Entity data changes (posts updated, deleted, etc.)
     * - List metadata items change (list structure changed)
     *
     * Use this for refreshing list contents in the UI.
     */
    fun addDataObserver(observer: () -> Unit) {
        dataObservers.add(observer)
    }

    /**
     * Add an observer for state changes (sync status changed).
     *
     * State observers are notified when the sync state changes:
     * - Idle -> FetchingFirstPage (refresh started)
     * - Idle -> FetchingNextPage (load more started)
     * - Fetching* -> Idle (sync completed)
     * - Fetching* -> Error (sync failed)
     *
     * Use this for updating loading indicators in the UI.
     */
    fun addStateObserver(observer: () -> Unit) {
        stateObservers.add(observer)
    }

    /**
     * Add an observer for both data and state changes.
     *
     * This is a convenience method that registers the observer for both
     * data and state updates. Use this when you want to refresh the entire
     * UI on any change.
     */
    fun addObserver(observer: () -> Unit) {
        dataObservers.add(observer)
        stateObservers.add(observer)
    }

    /**
     * Remove a data observer.
     */
    fun removeDataObserver(observer: () -> Unit) {
        dataObservers.remove(observer)
    }

    /**
     * Remove a state observer.
     */
    fun removeStateObserver(observer: () -> Unit) {
        stateObservers.remove(observer)
    }

    /**
     * Remove an observer from both data and state lists.
     */
    fun removeObserver(observer: () -> Unit) {
        dataObservers.remove(observer)
        stateObservers.remove(observer)
    }

    /**
     * Load all items with their current states and data.
     *
     * Returns items in list order with:
     * - `id`: The post ID
     * - `state`: Current fetch state (Missing, Fetching, Cached, Stale, Failed)
     * - `data`: Full entity data when state is Cached, null otherwise
     *
     * This is a suspend function that reads from cache/memory stores on a background thread.
     * Use the state to determine how to render each item in the UI.
     */
    suspend fun loadItems(): List<PostMetadataCollectionItem> = collection.loadItems()

    /**
     * Refresh the collection (fetch page 1, replace metadata).
     *
     * This:
     * 1. Fetches metadata from the network (page 1)
     * 2. Replaces existing metadata in the store
     * 3. Fetches missing/stale entities
     *
     * Returns sync statistics including counts and pagination info.
     *
     * This is a suspend function and should be called from a coroutine or background thread.
     */
    suspend fun refresh(): SyncResult = collection.refresh()

    /**
     * Load the next page of items.
     *
     * This:
     * 1. Fetches metadata for the next page
     * 2. Appends to existing metadata in the store
     * 3. Fetches missing/stale entities from the new page
     *
     * Returns a no-op result if already on the last page.
     *
     * This is a suspend function and should be called from a coroutine or background thread.
     */
    suspend fun loadNextPage(): SyncResult = collection.loadNextPage()

    /**
     * Check if there are more pages to load.
     */
    fun hasMorePages(): Boolean = collection.hasMorePages()

    /**
     * Get the current page number (0 = not loaded yet).
     */
    fun currentPage(): UInt = collection.currentPage()

    /**
     * Get the total number of pages, if known.
     */
    fun totalPages(): UInt? = collection.totalPages()

    /**
     * Get the current sync state for this collection.
     *
     * Returns:
     * - [ListState.IDLE] - No sync in progress
     * - [ListState.FETCHING_FIRST_PAGE] - Refresh in progress
     * - [ListState.FETCHING_NEXT_PAGE] - Load more in progress
     * - [ListState.ERROR] - Last sync failed
     *
     * Use this with state observers to show loading indicators in the UI.
     * This is a suspend function that reads from the database on a background thread.
     */
    suspend fun syncState(): ListState = collection.syncState()

    /**
     * Internal method called by DatabaseChangeNotifier when a database update occurs.
     *
     * Checks relevance and notifies appropriate observers:
     * - Data updates -> dataObservers
     * - State updates -> stateObservers
     */
    internal fun notifyIfRelevant(hook: UpdateHook) {
        val isDataRelevant = collection.isRelevantDataUpdate(hook)
        val isStateRelevant = collection.isRelevantStateUpdate(hook)
        if (isDataRelevant) {
            dataObservers.forEach { it() }
        }
        if (isStateRelevant) {
            stateObservers.forEach { it() }
        }
    }

    /**
     * Unregister this collection from receiving database change notifications.
     *
     * Call this when the collection is no longer needed, or use `.use { }` for automatic cleanup.
     * After calling close(), the collection will no longer notify observers of database changes.
     */
    override fun close() {
        DatabaseChangeNotifier.unregister(this)
    }
}
