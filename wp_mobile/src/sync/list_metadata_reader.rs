use super::EntityMetadata;
use wp_mobile_cache::list_metadata::ListState;

/// Combined list information: pagination + sync state.
///
/// Returned by a single JOIN query on `list_metadata` + `list_metadata_state` tables.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ListInfo {
    /// Current sync state (Idle, FetchingFirstPage, FetchingNextPage, Error)
    pub state: ListState,
    /// Error message if state is Error
    pub error_message: Option<String>,
    /// Current page that has been loaded (0 = no pages loaded)
    pub current_page: i64,
    /// Total number of pages from API response
    pub total_pages: Option<i64>,
    /// Total number of items from API response
    pub total_items: Option<i64>,
    /// Items per page
    pub per_page: i64,
}

/// Read-only access to list metadata.
///
/// This trait allows components (like `MetadataCollection`) to read list structure
/// without being able to modify it. Only the service layer should write metadata.
pub trait ListMetadataReader: Send + Sync {
    /// Get list info (pagination + state) in a single query.
    ///
    /// Returns `None` if no metadata has been stored for this key.
    fn get_list_info(&self, key: &str) -> Option<ListInfo>;

    /// Get the items for a list.
    ///
    /// Returns `None` if no metadata has been stored for this key.
    /// Returns `Some(vec![])` if the list exists but has no items.
    fn get_items(&self, key: &str) -> Option<Vec<EntityMetadata>>;
}
