package rs.wordpress.cache.kotlin

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
