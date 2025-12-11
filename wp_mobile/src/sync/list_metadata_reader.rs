use super::EntityMetadata;

/// Read-only access to list metadata.
///
/// This trait allows components (like `MetadataCollection`) to read list structure
/// without being able to modify it. Only the service layer should write metadata.
///
/// # Relevance Checking
///
/// The trait also provides methods for checking if database update hooks are relevant
/// to a specific collection. These are used to implement split observers for data vs
/// state updates.
///
/// Default implementations return `false` (safe for implementations that don't support
/// these checks). Database-backed implementations override with actual checks.
pub trait ListMetadataReader: Send + Sync {
    /// Get the metadata list for a filter key.
    ///
    /// Returns `None` if no metadata has been stored for this key.
    fn get(&self, key: &str) -> Option<Vec<EntityMetadata>>;

    /// Get the list_metadata_id (database rowid) for a given key.
    ///
    /// Returns `None` if no list exists for this key yet, or if this
    /// implementation doesn't support this operation.
    ///
    /// Used by collections to cache the ID for efficient state update matching.
    fn get_list_metadata_id(&self, _key: &str) -> Option<i64> {
        None
    }

    /// Check if a list_metadata_items row belongs to a specific key.
    ///
    /// Given a rowid from the list_metadata_items table (from an UpdateHook),
    /// returns true if that item row belongs to the given key.
    ///
    /// Default implementation returns `false`.
    fn is_item_row_for_key(&self, _item_row_id: i64, _key: &str) -> bool {
        false
    }

    /// Check if a list_metadata_state row belongs to a specific list_metadata_id.
    ///
    /// Given a rowid from the list_metadata_state table (from an UpdateHook),
    /// returns true if that state row belongs to the given list_metadata_id.
    ///
    /// Default implementation returns `false`.
    fn is_state_row_for_list(&self, _state_row_id: i64, _list_metadata_id: i64) -> bool {
        false
    }

    /// Get the current sync state for a list.
    ///
    /// Returns the current `ListState` (Idle, FetchingFirstPage, FetchingNextPage, Error).
    /// Used by UI to show loading indicators or error states.
    ///
    /// Default implementation returns `Idle`.
    fn get_sync_state(&self, _key: &str) -> wp_mobile_cache::list_metadata::ListState {
        wp_mobile_cache::list_metadata::ListState::Idle
    }
}
