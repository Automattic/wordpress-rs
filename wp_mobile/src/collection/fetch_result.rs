use std::sync::Arc;
use wp_mobile_cache::entity::EntityId;

/// Result of a network fetch operation
///
/// Contains the entity IDs of fetched items plus pagination metadata
/// from the API response. ViewModels use this to track pagination state.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FetchResult {
    /// Entity IDs of successfully fetched and cached items
    pub entity_ids: Vec<Arc<EntityId>>,

    /// Total number of items matching the query (from API)
    pub total_items: Option<u64>,

    /// Total number of pages available (from API)
    pub total_pages: Option<u32>,

    /// The page number that was fetched
    pub current_page: u32,
}
