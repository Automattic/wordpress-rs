package rs.wordpress.cache.kotlin

import uniffi.wp_mobile.AnyPostFilter
import uniffi.wp_mobile.FullEntityAnyPostWithEditContext
import uniffi.wp_mobile.PostService
import uniffi.wp_mobile_cache.EntityId

fun PostService.getObservableEntityWithEditContext(entityId: EntityId): ObservableEntity<FullEntityAnyPostWithEditContext> {
    val entity = this.getEntityWithEditContext(entityId)
    return createObservableEntity(
        loadData = entity::loadData,
        loadDataAsync = entity::loadDataAsync,
        id = entity::id,
        isRelevantUpdate = entity::isRelevantUpdate
    )
}

fun PostService.getObservableAllPostsWithEditContext(): ObservableCollection<FullEntityAnyPostWithEditContext> {
    val collection = this.getAllPostsWithEditContext()
    return createObservableCollection(
        loadData = collection::loadData,
        loadDataAsync = collection::loadDataAsync,
        isRelevantUpdate = collection::isRelevantUpdate
    )
}

fun PostService.getObservablePostCollectionWithEditContext(filter: AnyPostFilter): ObservableCollection<FullEntityAnyPostWithEditContext> {
    val collection = this.createPostCollectionWithEditContext(filter)
    return createObservableCollection(
        loadData = collection::loadData,
        loadDataAsync = collection::loadDataAsync,
        isRelevantUpdate = collection::isRelevantUpdate
    )
}
