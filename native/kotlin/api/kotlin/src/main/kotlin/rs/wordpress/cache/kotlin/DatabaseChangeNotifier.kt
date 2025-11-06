package rs.wordpress.cache.kotlin

import uniffi.wp_mobile_cache.DatabaseDelegate
import uniffi.wp_mobile_cache.UpdateHook
import java.util.concurrent.CopyOnWriteArraySet

/**
 * Global notifier that receives database updates and dispatches them to registered ObservableEntity instances.
 *
 * This singleton acts as a bridge between the Rust database update mechanism and Kotlin ObservableEntity wrappers.
 * It implements DatabaseDelegate to receive updates from WpApiCache, then notifies all registered entities
 * to check if the update is relevant to them.
 *
 * This design keeps database implementation details (table names, rowids) hidden from application code -
 * the Entity's is_relevant_update closure handles all the matching logic in Rust.
 */
object DatabaseChangeNotifier : DatabaseDelegate {
    private val observableEntities = CopyOnWriteArraySet<ObservableEntity<*>>()

    /**
     * Register an ObservableEntity to receive database change notifications.
     *
     * The entity will be notified of all database updates and can decide internally
     * whether the update is relevant to it.
     */
    fun register(entity: ObservableEntity<*>) {
        observableEntities.add(entity)
    }

    /**
     * Unregister an ObservableEntity from receiving database change notifications.
     */
    fun unregister(entity: ObservableEntity<*>) {
        observableEntities.remove(entity)
    }

    /**
     * Called by WpApiCache when a database update occurs.
     *
     * Notifies all registered ObservableEntity instances, which will check if the update
     * is relevant to them using Entity.is_relevant_update().
     */
    override fun didUpdate(updateHook: UpdateHook) {
        observableEntities.forEach { it.notifyIfRelevant(updateHook) }
    }
}
