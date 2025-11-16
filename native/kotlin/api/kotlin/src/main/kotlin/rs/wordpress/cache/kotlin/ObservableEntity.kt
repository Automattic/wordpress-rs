package rs.wordpress.cache.kotlin

import uniffi.wp_mobile_cache.EntityId
import uniffi.wp_mobile_cache.UpdateHook
import java.util.concurrent.CopyOnWriteArrayList

/**
 * Create an observable entity that notifies observers when data changes.
 *
 * This helper automatically registers the entity with [DatabaseChangeNotifier].
 * Use service extension functions (e.g., `getObservableEntityWithEditContext`)
 * instead of calling this directly.
 *
 * Example:
 * ```
 * val observablePost = postService.getObservableEntityWithEditContext(postId)
 * observablePost.addObserver {
 *     val updatedData = observablePost.loadData()
 *     // React to changes
 * }
 * ```
 */
fun <D> createObservableEntity(
    loadData: suspend () -> D?,
    id: () -> EntityId,
    isRelevantUpdate: (UpdateHook) -> Boolean
): ObservableEntity<D> = ObservableEntity(
    loadDataFn = loadData,
    idFn = id,
    isRelevantUpdateFn = isRelevantUpdate
).also {
    DatabaseChangeNotifier.register(it)
}

/**
 * Observable wrapper around entity data that notifies observers when changes occur.
 *
 * Bridges Rust entity updates to Kotlin observer pattern without exposing database
 * implementation details.
 *
 * Generic over data type `D` (e.g., `FullEntityAnyPostWithEditContext`).
 *
 * Create instances using [createObservableEntity] or service extension functions
 * rather than the constructor directly.
 */
class ObservableEntity<D>(
    private val loadDataFn: suspend () -> D?,
    private val idFn: () -> EntityId,
    private val isRelevantUpdateFn: (UpdateHook) -> Boolean
) {
    private val observers = CopyOnWriteArrayList<() -> Unit>()

    /**
     * Add an observer to be notified when entity data changes.
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
     * Subsequent calls may return different results if the underlying data has changed.
     *
     * This is a suspend function and should be called from a coroutine or background thread.
     */
    suspend fun loadData(): D? = loadDataFn()

    /**
     * Get the entity's ID.
     */
    fun id(): EntityId = idFn()

    /**
     * Internal method called by DatabaseChangeNotifier when a database update occurs.
     *
     * Checks if the update is relevant to this entity, and if so, notifies all observers.
     */
    internal fun notifyIfRelevant(hook: UpdateHook) {
        if (isRelevantUpdateFn(hook)) {
            observers.forEach { it() }
        }
    }
}
