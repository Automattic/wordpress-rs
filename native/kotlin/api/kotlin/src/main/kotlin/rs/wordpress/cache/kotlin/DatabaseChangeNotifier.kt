package rs.wordpress.cache.kotlin

import uniffi.wp_mobile_cache.DatabaseDelegate
import uniffi.wp_mobile_cache.UpdateHook
import java.util.concurrent.CopyOnWriteArraySet

/**
 * Global notifier that receives database updates and dispatches them to registered observables.
 *
 * This singleton acts as a bridge between the Rust database update mechanism and Kotlin observable wrappers.
 * It implements DatabaseDelegate to receive updates from WpApiCache, then notifies all registered
 * entities and collections to check if the update is relevant to them.
 *
 * This design keeps database implementation details (table names, rowids) hidden from application code -
 * the observable's is_relevant_update closure handles all the matching logic in Rust.
 */
object DatabaseChangeNotifier : DatabaseDelegate {
    private val observableEntities = CopyOnWriteArraySet<ObservableEntity<*>>()
    private val observableCollections = CopyOnWriteArraySet<ObservableCollection<*>>()

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
     * Register an ObservableCollection to receive database change notifications.
     *
     * The collection will be notified of all database updates and can decide internally
     * whether the update is relevant to it.
     */
    fun register(collection: ObservableCollection<*>) {
        observableCollections.add(collection)
    }

    /**
     * Unregister an ObservableCollection from receiving database change notifications.
     */
    fun unregister(collection: ObservableCollection<*>) {
        observableCollections.remove(collection)
    }

    /**
     * Called by WpApiCache when a database update occurs.
     *
     * Notifies all registered observables (entities and collections), which will check if the update
     * is relevant to them using their is_relevant_update() methods.
     */
    override fun didUpdate(updateHook: UpdateHook) {
        observableEntities.forEach { it.notifyIfRelevant(updateHook) }
        observableCollections.forEach { it.notifyIfRelevant(updateHook) }
    }
}
