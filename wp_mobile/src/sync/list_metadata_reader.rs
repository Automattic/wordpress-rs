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
    ///
    /// Default implementation returns `Idle`.
    fn get_sync_state(&self, _key: &str) -> wp_mobile_cache::list_metadata::ListState {
        wp_mobile_cache::list_metadata::ListState::Idle
    }

    /// Get the current page number for a list.
    ///
    /// Returns 0 if no pages have been fetched yet.
    /// Default implementation returns 0.
    fn get_current_page(&self, _key: &str) -> i64 {
        0
    }

    /// Get the total number of pages for a list.
    ///
    /// Returns `None` if unknown (no fetch has completed yet).
    /// Default implementation returns `None`.
    fn get_total_pages(&self, _key: &str) -> Option<i64> {
        None
    }

    /// Get the total number of items for a list.
    ///
    /// Returns `None` if unknown (no fetch has completed yet).
    /// Default implementation returns `None`.
    fn get_total_items(&self, _key: &str) -> Option<i64> {
        None
    }

    /// Get the items per page setting for a list.
    ///
    /// Returns the configured per_page value, or 20 as default.
    /// Default implementation returns 20.
    fn get_per_page(&self, _key: &str) -> i64 {
        20
    }
}
