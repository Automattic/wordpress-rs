package rs.wordpress.cache.kotlin

import uniffi.wp_mobile.PostMetadataCollectionItem
import uniffi.wp_mobile.PostMetadataCollectionWithEditContext
import uniffi.wp_mobile.SyncResult
import uniffi.wp_mobile_cache.UpdateHook
import java.util.concurrent.CopyOnWriteArrayList

// TODO: Move state representation to Rust with proper enum modeling.
// See metadata_collection_v3.md "TODO: Refined State Representation"
// Current design uses separate fields (id, state, data); should be a sealed class for type safety.
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
class ObservableMetadataCollection(
    private val collection: PostMetadataCollectionWithEditContext
) : AutoCloseable {
    private val observers = CopyOnWriteArrayList<() -> Unit>()

    /**
     * Add an observer to be notified when collection data changes.
     *
     * Observers are called when a relevant database update occurs.
     * The observer is a simple callback - it doesn't receive the new data,
     * just a notification to re-read via [loadItems].
     */
    fun addObserver(observer: () -> Unit) {
        observers.add(observer)
    }

    /**
     * Remove a previously added observer.
     */
    fun removeObserver(observer: () -> Unit) {
        observers.remove(observer)
    }

    /**
     * Load all items with their current states and data.
     *
     * Returns items in list order with:
     * - `id`: The post ID
     * - `state`: Current fetch state (Missing, Fetching, Cached, Stale, Failed)
     * - `data`: Full entity data when state is Cached, null otherwise
     *
     * This is a synchronous operation that reads from cache/memory stores.
     * Use the state to determine how to render each item in the UI.
     */
    fun loadItems(): List<PostMetadataCollectionItem> = collection.loadItems()

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
     * Internal method called by DatabaseChangeNotifier when a database update occurs.
     *
     * Checks if the update is relevant to this collection, and if so, notifies all observers.
     */
    internal fun notifyIfRelevant(hook: UpdateHook) {
        if (collection.isRelevantUpdate(hook)) {
            observers.forEach { it() }
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
