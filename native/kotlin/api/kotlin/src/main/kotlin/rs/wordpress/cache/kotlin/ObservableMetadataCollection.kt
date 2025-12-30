package rs.wordpress.cache.kotlin

import uniffi.wp_mobile.ListInfo
import uniffi.wp_mobile.PostMetadataCollectionItem
import uniffi.wp_mobile.PostMetadataCollectionWithEditContext
import uniffi.wp_mobile.SyncResult
import uniffi.wp_mobile_cache.ListState
import uniffi.wp_mobile_cache.UpdateHook
import java.util.concurrent.CopyOnWriteArrayList

/**
 * Check if there are more pages to load.
 *
 * Returns `true` if:
 * - No metadata loaded yet (assume more)
 * - Total pages is unknown (assume more)
 * - Current page is less than total pages
 */
val ListInfo.hasMorePages: Boolean
    get() {
        val current = currentPage ?: return true // No pages loaded yet
        val total = totalPages ?: return true // Total unknown, assume more
        return current < total
    }

/**
 * Check if a sync operation is in progress.
 */
val ListInfo.isSyncing: Boolean
    get() = state == ListState.FETCHING_FIRST_PAGE || state == ListState.FETCHING_NEXT_PAGE

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
    private val listInfoObservers = CopyOnWriteArrayList<() -> Unit>()

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
     * Add an observer for list info changes (pagination or sync state changed).
     *
     * ListInfo observers are notified when:
     * - Pagination info changes (current page, total pages updated after fetch)
     * - Sync state changes (Idle -> FetchingFirstPage, etc.)
     *
     * Use this for updating pagination display and loading indicators in the UI.
     */
    fun addListInfoObserver(observer: () -> Unit) {
        listInfoObservers.add(observer)
    }

    /**
     * Add an observer for both data and list info changes.
     *
     * This is a convenience method that registers the observer for both
     * data and list info updates. Use this when you want to refresh the entire
     * UI on any change.
     */
    fun addObserver(observer: () -> Unit) {
        dataObservers.add(observer)
        listInfoObservers.add(observer)
    }

    /**
     * Remove a data observer.
     */
    fun removeDataObserver(observer: () -> Unit) {
        dataObservers.remove(observer)
    }

    /**
     * Remove a list info observer.
     */
    fun removeListInfoObserver(observer: () -> Unit) {
        listInfoObservers.remove(observer)
    }

    /**
     * Remove an observer from both data and list info lists.
     */
    fun removeObserver(observer: () -> Unit) {
        dataObservers.remove(observer)
        listInfoObservers.remove(observer)
    }

    /**
     * Load all items with their current states and data.
     *
     * Returns items in list order with type-safe state representation.
     * Each item's `state` is a [PostItemState] sealed class that encodes both
     * sync status and data availability:
     *
     * - `Cached`: Fresh data, no fetch needed
     * - `Stale`: Outdated data, could benefit from refresh
     * - `FetchingWithData`: Refresh in progress, showing cached data
     * - `FailedWithData`: Fetch failed, showing last known data
     * - `Missing`: Needs fetch, no cached data
     * - `Fetching`: Fetch in progress, no cached data
     * - `Failed`: Fetch failed, no cached data
     *
     * This is a suspend function that reads from cache on a background thread.
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
     * Get combined list info (pagination + sync state) in a single query.
     *
     * Returns `null` if the list hasn't been created yet.
     *
     * The returned [ListInfo] contains:
     * - `state`: Current sync state (IDLE, FETCHING_FIRST_PAGE, FETCHING_NEXT_PAGE, ERROR)
     * - `errorMessage`: Error message if state is ERROR
     * - `currentPage`: Current page number (null = not loaded yet)
     * - `totalPages`: Total pages if known
     * - `totalItems`: Total items if known
     * - `perPage`: Items per page setting
     *
     * Use [ListInfo.hasMorePages] extension to check if more pages are available.
     */
    fun listInfo(): ListInfo? = collection.listInfo()

    /**
     * Internal method called by DatabaseChangeNotifier when a database update occurs.
     *
     * Checks relevance and notifies appropriate observers:
     * - Data updates -> dataObservers
     * - List info updates -> listInfoObservers
     */
    internal fun notifyIfRelevant(hook: UpdateHook) {
        val isDataRelevant = collection.isRelevantDataUpdate(hook)
        val isListInfoRelevant = collection.isRelevantListInfoUpdate(hook)
        if (isDataRelevant) {
            dataObservers.forEach { it() }
        }
        if (isListInfoRelevant) {
            listInfoObservers.forEach { it() }
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
