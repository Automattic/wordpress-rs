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
 * Example:
 * ```
 * val observablePosts = postService.getObservableAllPostsWithEditContext()
 * observablePosts.addObserver {
 *     val allPosts = observablePosts.loadData()
 *     // React to changes
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
 */
class ObservableCollection<D>(
    private val loadDataFn: suspend () -> List<D>,
    private val isRelevantUpdateFn: (UpdateHook) -> Boolean
) {
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
}
