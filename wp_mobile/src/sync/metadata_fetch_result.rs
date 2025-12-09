use std::hash::Hash;

use super::EntityMetadata;

/// Result of a metadata fetch operation.
///
/// Contains lightweight metadata (id + modified_gmt) for entities,
/// plus pagination info from the API response.
#[derive(Debug, Clone)]
pub struct MetadataFetchResult<Id>
where
    Id: Clone + Eq + Hash,
{
    /// Metadata for entities in this page
    pub metadata: Vec<EntityMetadata<Id>>,

    /// Total number of items matching the query (from API)
    pub total_items: Option<i64>,

    /// Total number of pages available (from API)
    pub total_pages: Option<u32>,

    /// The page number that was fetched
    pub current_page: u32,
}
