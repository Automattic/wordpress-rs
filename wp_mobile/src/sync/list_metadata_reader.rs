use super::EntityMetadata;

/// Read-only access to list metadata.
///
/// This trait allows components (like `MetadataCollection`) to read list structure
/// without being able to modify it. Only the service layer should write metadata.
pub trait ListMetadataReader: Send + Sync {
    /// Get the metadata list for a filter key.
    ///
    /// Returns `None` if no metadata has been stored for this key.
    fn get(&self, key: &str) -> Option<Vec<EntityMetadata>>;

    /// Get the current sync state for a list.
    ///
    /// Returns the current `ListState` (Idle, FetchingFirstPage, FetchingNextPage, Error).
    /// Used by UI to show loading indicators or error states.
    fn get_sync_state(&self, key: &str) -> wp_mobile_cache::list_metadata::ListState;

    /// Get the current page number for a list.
    ///
    /// Returns 0 if no pages have been fetched yet.
    fn get_current_page(&self, key: &str) -> i64;

    /// Get the total number of pages for a list.
    ///
    /// Returns `None` if unknown (no fetch has completed yet).
    fn get_total_pages(&self, key: &str) -> Option<i64>;

    /// Get the total number of items for a list.
    ///
    /// Returns `None` if unknown (no fetch has completed yet).
    fn get_total_items(&self, key: &str) -> Option<i64>;

    /// Get the items per page setting for a list.
    ///
    /// Returns the configured per_page value, or a default if not set.
    fn get_per_page(&self, key: &str) -> i64;
}
