package rs.wordpress.cache.kotlin

import uniffi.wp_mobile.PostService

/**
 * Get an observable entity handle for a specific post with edit context.
 *
 * Returns an ObservableEntity that can be used to:
 * - Read post data with full edit context via loadData()
 * - Observe changes via addObserver()
 *
 * The returned entity is automatically registered with DatabaseChangeNotifier
 * to receive database updates.
 *
 * Usage:
 * ```
 * val observablePost = postService.getObservableEntityWithEditContext(postId)
 * observablePost.addObserver {
 *     val updatedData = observablePost.loadData()
 *     // React to changes
 * }
 * ```
 */
fun PostService.getObservableEntityWithEditContext(id: Long): ObservableEntity {
    val entity = this.getEntityWithEditContext(id)
    val observableEntity = ObservableEntity(entity)
    DatabaseChangeNotifier.register(observableEntity)
    return observableEntity
}
