package rs.wordpress.cache.kotlin

import uniffi.wp_mobile_cache.UpdateHook
import java.util.concurrent.CopyOnWriteArrayList

/**
 * Create an observable collection that notifies observers when data changes.
 *
 * This helper automatically registers the collection with [DatabaseChangeNotifier].
 * Use service extension functions (e.g., `getObservableAllPostsWithEditContext`)
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
 *     private val observablePosts = postService.getObservableAllPostsWithEditContext()
 *
 *     init {
 *         observablePosts.addObserver { /* update UI */ }
 *     }
 *
 *     override fun onCleared() {
 *         super.onCleared()
 *         observablePosts.close()
 *     }
 * }
 * ```
 */
fun <D> createObservableCollection(
    loadData: suspend () -> List<D>,
    isRelevantUpdate: (UpdateHook) -> Boolean
): ObservableCollection<D> = ObservableCollection(
    loadDataFn = loadData,
    isRelevantUpdateFn = isRelevantUpdate
).also {
    DatabaseChangeNotifier.register(it)
}

/**
 * Observable wrapper around collection data that notifies observers when changes occur.
 *
 * Similar to [ObservableEntity] but for collections of items rather than individual entities.
 * Uses table-level filtering - any insert, update, or delete to the table triggers observers.
 *
 * This is a pragmatic design for observing many items at once. When any change occurs,
 * the entire collection is re-queried rather than tracking individual item changes.
 *
 * Generic over data type `D` (e.g., `FullEntityAnyPostWithEditContext`).
 *
 * Create instances using [createObservableCollection] or service extension functions
 * rather than the constructor directly.
 *
 * Implements [AutoCloseable] to support cleanup. Call [close] when done (typically in
 * ViewModel.onCleared()) to unregister from [DatabaseChangeNotifier].
 */
class ObservableCollection<D>(
    private val loadDataFn: suspend () -> List<D>,
    private val isRelevantUpdateFn: (UpdateHook) -> Boolean
) : AutoCloseable {
    private val observers = CopyOnWriteArrayList<() -> Unit>()

    /**
     * Add an observer to be notified when collection data changes.
     *
     * Observers are called when a relevant database update occurs.
     * The observer is a simple callback - it doesn't receive the new data,
     * just a notification to re-read via loadData().
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
     * Load current data from cache/DB.
     *
     * **Important**: This is an expensive operation that reads from the database each time.
     * The entire collection is re-queried on every call (stateless behavior).
     *
     * Returns all items in the collection.
     * This is a suspend function and should be called from a coroutine or background thread.
     */
    suspend fun loadData(): List<D> = loadDataFn()

    /**
     * Internal method called by DatabaseChangeNotifier when a database update occurs.
     *
     * Checks if the update is relevant to this collection, and if so, notifies all observers.
     */
    internal fun notifyIfRelevant(hook: UpdateHook) {
        if (isRelevantUpdateFn(hook)) {
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
