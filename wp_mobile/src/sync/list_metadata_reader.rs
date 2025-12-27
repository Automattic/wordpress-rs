use super::EntityMetadata;
use wp_mobile_cache::list_metadata::{ListKey, ListState};

/// Combined list information: pagination + sync state.
///
/// Returned by a single JOIN query on `list_metadata` + `list_metadata_state` tables.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ListInfo {
    /// Current sync state (Idle, FetchingFirstPage, FetchingNextPage, Error)
    pub state: ListState,
    /// Error message if state is Error
    pub error_message: Option<String>,
    /// Current page that has been loaded
    ///
    /// `None` means no pages have been loaded yet.
    #[uniffi(default = None)]
    pub current_page: Option<u32>,
    /// Total number of pages from API response
    #[uniffi(default = None)]
    pub total_pages: Option<u32>,
    /// Total number of items from API response
    #[uniffi(default = None)]
    pub total_items: Option<i64>,
    /// Items per page
    pub per_page: u32,
}

/// Read-only access to list metadata.
///
/// This trait allows components (like `MetadataCollection`) to read list structure
/// without being able to modify it. Only the service layer should write metadata.
pub trait ListMetadataReader: Send + Sync {
    /// Get list info (pagination + state) in a single query.
    ///
    /// Returns `None` if no metadata has been stored for this key.
    fn get_list_info(&self, key: &ListKey) -> Option<ListInfo>;

    /// Get the items for a list.
    ///
    /// Returns `None` if no metadata has been stored for this key.
    /// Returns `Some(vec![])` if the list exists but has no items.
    fn get_items(&self, key: &ListKey) -> Option<Vec<EntityMetadata>>;
}
