use std::collections::HashMap;
use std::sync::RwLock;

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
/// Default implementations return `false` (safe for in-memory stores that don't support
/// these checks). Database-backed implementations override with actual checks.
pub trait ListMetadataReader: Send + Sync {
    /// Get the metadata list for a filter key.
    ///
    /// Returns `None` if no metadata has been stored for this key.
    fn get(&self, key: &str) -> Option<Vec<EntityMetadata>>;

    /// Get the list_metadata_id (database rowid) for a given key.
    ///
    /// Returns `None` if no list exists for this key yet, or if this is an
    /// in-memory implementation that doesn't support this operation.
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
    /// Default implementation returns `false` (in-memory stores don't track row IDs).
    fn is_item_row_for_key(&self, _item_row_id: i64, _key: &str) -> bool {
        false
    }

    /// Check if a list_metadata_state row belongs to a specific list_metadata_id.
    ///
    /// Given a rowid from the list_metadata_state table (from an UpdateHook),
    /// returns true if that state row belongs to the given list_metadata_id.
    ///
    /// Default implementation returns `false` (in-memory stores don't track row IDs).
    fn is_state_row_for_list(&self, _state_row_id: i64, _list_metadata_id: i64) -> bool {
        false
    }

    /// Get the current sync state for a list.
    ///
    /// Returns the current `ListState` (Idle, FetchingFirstPage, FetchingNextPage, Error).
    /// Used by UI to show loading indicators or error states.
    ///
    /// Default implementation returns `Idle` (in-memory stores don't track state).
    fn get_sync_state(&self, _key: &str) -> wp_mobile_cache::list_metadata::ListState {
        wp_mobile_cache::list_metadata::ListState::Idle
    }
}

/// Store for list metadata (entity IDs + modified timestamps per filter).
///
/// Maps filter keys (e.g., "site_1:publish:date_desc") to ordered lists of
/// `EntityMetadata`. This defines the list structure for each filter.
///
/// This is a memory-only store - metadata resets on app restart.
/// Can be swapped for a persistent implementation later.
pub struct ListMetadataStore {
    data: RwLock<HashMap<String, Vec<EntityMetadata>>>,
}

impl ListMetadataStore {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }

    /// Set (replace) metadata for a filter key.
    ///
    /// Use this when fetching the first page to replace any existing metadata.
    pub fn set(&self, key: &str, metadata: Vec<EntityMetadata>) {
        self.data
            .write()
            .expect("RwLock poisoned")
            .insert(key.to_string(), metadata);
    }

    /// Append metadata to existing list for a filter key.
    ///
    /// Use this when fetching subsequent pages to add to the list.
    /// If the key doesn't exist, creates a new list.
    pub fn append(&self, key: &str, metadata: Vec<EntityMetadata>) {
        self.data
            .write()
            .expect("RwLock poisoned")
            .entry(key.to_string())
            .or_default()
            .extend(metadata);
    }

    /// Remove metadata for a filter key.
    pub fn remove(&self, key: &str) {
        self.data.write().expect("RwLock poisoned").remove(key);
    }

    /// Check if a filter key exists.
    pub fn contains(&self, key: &str) -> bool {
        self.data.read().expect("RwLock poisoned").contains_key(key)
    }

    /// Clear all metadata.
    pub fn clear(&self) {
        self.data.write().expect("RwLock poisoned").clear();
    }

    /// Get the number of stored filter keys.
    pub fn len(&self) -> usize {
        self.data.read().expect("RwLock poisoned").len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.data.read().expect("RwLock poisoned").is_empty()
    }
}

impl Default for ListMetadataStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ListMetadataReader for ListMetadataStore {
    fn get(&self, key: &str) -> Option<Vec<EntityMetadata>> {
        self.data.read().expect("RwLock poisoned").get(key).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wp_api::prelude::WpGmtDateTime;

    fn test_metadata(id: i64) -> EntityMetadata {
        EntityMetadata::with_modified(id, WpGmtDateTime::from_timestamp(1000 + id))
    }

    #[test]
    fn test_get_returns_none_for_unknown() {
        let store = ListMetadataStore::new();
        assert!(store.get("unknown_key").is_none());
    }

    #[test]
    fn test_set_and_get() {
        let store = ListMetadataStore::new();
        let metadata = vec![test_metadata(1), test_metadata(2)];

        store.set("posts:publish", metadata.clone());

        let result = store.get("posts:publish");
        assert_eq!(result, Some(metadata));
    }

    #[test]
    fn test_set_replaces_existing() {
        let store = ListMetadataStore::new();

        store.set("key", vec![test_metadata(1)]);
        store.set("key", vec![test_metadata(2), test_metadata(3)]);

        let result = store.get("key").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, 2);
        assert_eq!(result[1].id, 3);
    }

    #[test]
    fn test_append_to_existing() {
        let store = ListMetadataStore::new();

        store.set("key", vec![test_metadata(1)]);
        store.append("key", vec![test_metadata(2), test_metadata(3)]);

        let result = store.get("key").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[1].id, 2);
        assert_eq!(result[2].id, 3);
    }

    #[test]
    fn test_append_creates_new_if_missing() {
        let store = ListMetadataStore::new();

        store.append("key", vec![test_metadata(1)]);

        let result = store.get("key").unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_remove() {
        let store = ListMetadataStore::new();

        store.set("key", vec![test_metadata(1)]);
        assert!(store.contains("key"));

        store.remove("key");
        assert!(!store.contains("key"));
        assert!(store.get("key").is_none());
    }

    #[test]
    fn test_clear() {
        let store = ListMetadataStore::new();

        store.set("key1", vec![test_metadata(1)]);
        store.set("key2", vec![test_metadata(2)]);
        assert_eq!(store.len(), 2);

        store.clear();
        assert!(store.is_empty());
    }

    #[test]
    fn test_reader_trait() {
        let store = ListMetadataStore::new();
        let metadata = vec![test_metadata(1)];
        store.set("key", metadata.clone());

        // Access via trait
        let reader: &dyn ListMetadataReader = &store;
        assert_eq!(reader.get("key"), Some(metadata));
        assert!(reader.get("unknown").is_none());
    }
}
