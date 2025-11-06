package rs.wordpress.cache.kotlin

import uniffi.wp_mobile_cache.UpdateHook
import java.util.concurrent.CopyOnWriteArrayList

/**
 * Observable wrapper for Entity that notifies observers when the underlying data changes.
 *
 * This class bridges Rust entity updates to Kotlin observer pattern without exposing
 * database implementation details (table names, rowids, etc).
 *
 * Generic over the data type D (e.g., FullEntityAnyPostWithEditContext).
 *
 * Usage:
 * ```
 * val observableEntity = ObservableEntity(
 *     loadDataFn = { entity.loadData() },
 *     loadDataAsyncFn = { entity.loadDataAsync() },
 *     idFn = { entity.id() },
 *     isRelevantUpdateFn = { hook -> entity.isRelevantUpdate(hook) }
 * )
 * observableEntity.addObserver {
 *     // React to changes
 *     val newData = observableEntity.loadData()
 * }
 * DatabaseChangeNotifier.register(observableEntity)
 * ```
 */
class ObservableEntity<D>(
    private val loadDataFn: () -> D?,
    private val loadDataAsyncFn: suspend () -> D?,
    private val idFn: () -> Long,
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
     * This is an expensive operation that reads from the database each time.
     */
    fun loadData(): D? = loadDataFn()

    /**
     * Load current data from cache/DB (async version).
     *
     * This is an expensive operation that reads from the database each time.
     * Use this version to avoid blocking the caller.
     */
    suspend fun loadDataAsync(): D? = loadDataAsyncFn()

    /**
     * Get the entity's ID.
     */
    fun id(): Long = idFn()

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
