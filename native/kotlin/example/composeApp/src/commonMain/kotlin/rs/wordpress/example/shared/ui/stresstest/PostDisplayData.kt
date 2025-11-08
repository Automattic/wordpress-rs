package rs.wordpress.example.shared.ui.stresstest

import uniffi.wp_mobile_cache.EntityId

data class PostDisplayData(
    val entityId: EntityId,
    val title: String,
    val contentPreview: String,
    val status: String,
    val author: String?,
    val date: String,
    val modified: String,
    val updateCount: Int = 0
)
