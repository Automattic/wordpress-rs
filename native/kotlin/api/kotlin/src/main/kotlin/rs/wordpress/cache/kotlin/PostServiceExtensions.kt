package rs.wordpress.cache.kotlin

import uniffi.wp_api.PostEndpointType
import uniffi.wp_mobile.AnyPostFilter
import uniffi.wp_mobile.FullEntityAnyPostWithEditContext
import uniffi.wp_mobile.PostListFilter
import uniffi.wp_mobile.PostService
import uniffi.wp_mobile_cache.EntityId

fun PostService.getObservableEntityWithEditContext(
    entityId: EntityId
): ObservableEntity<FullEntityAnyPostWithEditContext> {
    val entity = this.getEntityWithEditContext(entityId)
    return createObservableEntity(
        loadData = entity::loadData,
        id = entity::id,
        isRelevantUpdate = entity::isRelevantUpdate
    )
}

@Suppress("MaxLineLength")
fun PostService.getObservableAllPostsWithEditContext(): ObservableCollection<FullEntityAnyPostWithEditContext> {
    val collection = this.getAllPostsWithEditContext()
    return createObservableCollection(
        loadData = collection::loadData,
        isRelevantUpdate = collection::isRelevantUpdate
    )
}

fun PostService.getObservablePostCollectionWithEditContext(
    filter: AnyPostFilter
): ObservableCollection<FullEntityAnyPostWithEditContext> {
    val collection = this.createPostCollectionWithEditContext(filter)
    return createObservableCollection(
        loadData = collection::loadData,
        isRelevantUpdate = collection::isRelevantUpdate
    )
}

/**
 * Create an observable metadata collection for posts with edit context.
 *
 * This uses the "metadata-first" sync strategy:
 * 1. Fetch lightweight metadata (id + modified_gmt) to define list structure
 * 2. Selectively fetch full data for missing or stale items
 *
 * Unlike [getObservablePostCollectionWithEditContext] which fetches full data for all items,
 * this collection shows cached items immediately and fetches only what's needed.
 *
 * Items include fetch state (Missing, Fetching, Cached, Stale, Failed) so the UI
 * can show appropriate feedback for each item.
 *
 * @param endpointType The post endpoint type (Posts, Pages, or Custom)
 * @param filter Filter parameters (status, author, categories, etc.)
 * @return Observable metadata collection that notifies on database changes
 */
fun PostService.getObservablePostMetadataCollectionWithEditContext(
    endpointType: PostEndpointType,
    filter: PostListFilter
): ObservableMetadataCollection {
    val collection = this.createPostMetadataCollectionWithEditContext(endpointType, filter)
    return createObservableMetadataCollection(collection)
}
