use std::sync::Arc;

use wp_mobile_cache::{DbTable, UpdateHook, list_metadata::ListKey};

use super::{CollectionItem, EntityStateReader, ListInfo, ListMetadataReader};

/// Core collection infrastructure for metadata-first fetching.
///
/// This provides the shared query logic for all entity-specific collections:
/// - Items with their current fetch states
/// - List info (pagination + sync state)
/// - Database update relevance checking
///
/// Entity-specific collections compose this core and add their own fields
/// (filter, service reference, etc.) and sync logic.
///
/// See `PostMetadataCollectionWithEditContext` for an example.
pub struct MetadataCollectionCore {
    /// Key for metadata store lookup
    key: ListKey,

    /// Read-only access to list metadata
    metadata_reader: Arc<dyn ListMetadataReader>,

    /// Read-only access to entity states
    state_reader: Arc<dyn EntityStateReader>,

    /// Tables to monitor for data updates (entity tables like PostsEditContext)
    relevant_data_tables: Vec<DbTable>,

    /// Items per page configuration (default: 20)
    per_page: u32,
}

impl MetadataCollectionCore {
    /// Create a new metadata collection core.
    ///
    /// # Arguments
    /// * `key` - Key for metadata store lookup (e.g., "site_1:posts:publish")
    /// * `metadata_reader` - Read-only access to list metadata store
    /// * `state_reader` - Read-only access to entity state store
    /// * `relevant_data_tables` - DB tables to monitor for data updates (entity tables)
    pub fn new(
        key: ListKey,
        metadata_reader: Arc<dyn ListMetadataReader>,
        state_reader: Arc<dyn EntityStateReader>,
        relevant_data_tables: Vec<DbTable>,
    ) -> Self {
        Self {
            key,
            metadata_reader,
            state_reader,
            relevant_data_tables,
            per_page: 20,
        }
    }

    /// Set the number of items per page.
    ///
    /// Default is 20. Call this before `refresh()` if you need a different page size.
    pub fn with_per_page(mut self, per_page: u32) -> Self {
        self.per_page = per_page;
        self
    }

    /// Get the key for metadata store lookup.
    pub fn key(&self) -> &ListKey {
        &self.key
    }

    /// Get the number of items per page.
    pub fn per_page(&self) -> u32 {
        self.per_page
    }

    /// Get current items with their states.
    ///
    /// Returns a `CollectionItem` for each entity in the list, combining
    /// the metadata with the current fetch state.
    pub fn items(&self) -> Vec<CollectionItem> {
        self.metadata_reader
            .get_items(&self.key)
            .unwrap_or_default()
            .into_iter()
            .map(|metadata| {
                CollectionItem::new(metadata.clone(), self.state_reader.get(metadata.id))
            })
            .collect()
    }

    /// Get the combined list info (pagination + sync state) in a single query.
    ///
    /// Returns `None` if no metadata has been stored for this key.
    pub fn list_info(&self) -> Option<ListInfo> {
        self.metadata_reader.get_list_info(&self.key)
    }

    /// Get the current sync state for this collection.
    ///
    /// Returns the current `ListState`:
    /// - `Idle` - No sync in progress
    /// - `FetchingFirstPage` - Refresh in progress
    /// - `FetchingNextPage` - Load more in progress
    /// - `Error` - Last sync failed
    ///
    /// Use this to show loading indicators in the UI.
    pub fn sync_state(&self) -> wp_mobile_cache::list_metadata::ListState {
        self.list_info().map(|info| info.state).unwrap_or_default()
    }

    /// Check if a database update is relevant to this collection (either data or list info).
    ///
    /// Returns `true` if the update affects either data or list info.
    /// For more granular control, use `is_relevant_data_update` or `is_relevant_list_info_update`.
    pub fn is_relevant_update(&self, hook: &UpdateHook) -> bool {
        self.is_relevant_data_update(hook) || self.is_relevant_list_info_update(hook)
    }

    /// Check if a database update affects this collection's data.
    ///
    /// Returns `true` if the update is to:
    /// - An entity table this collection monitors (e.g., PostsEditContext, TermRelationships)
    /// - The ListMetadataItems table (any row - we can't filter by key without deadlocking)
    ///
    /// Use this for data observers that should refresh list contents.
    ///
    /// Note: We intentionally don't query the database here to avoid deadlocks when
    /// the hook fires during a transaction. This means we may get false positives for
    /// ListMetadataItems updates from other collections, but that's safe (just extra refreshes).
    pub fn is_relevant_data_update(&self, hook: &UpdateHook) -> bool {
        // Check entity tables
        if self.relevant_data_tables.contains(&hook.table) {
            return true;
        }

        // Check ListMetadataItems - return true for any update to avoid deadlock
        // (we can't query the DB to check if it's our key during a hook callback)
        if hook.table == DbTable::ListMetadataItems {
            return true;
        }

        false
    }

    /// Check if a database update affects this collection's list info (pagination + state).
    ///
    /// Returns `true` if the update is to:
    /// - `ListMetadata` table (pagination info changed)
    /// - `ListMetadataState` table (sync state changed)
    ///
    /// Use this for listInfo observers that should update pagination display and loading indicators.
    ///
    /// Note: We intentionally don't query the database here to avoid deadlocks when
    /// the hook fires during a transaction. This means we may get false positives for
    /// updates from other collections, but that's safe (just extra reads).
    pub fn is_relevant_list_info_update(&self, hook: &UpdateHook) -> bool {
        // Just check the table - don't query DB to avoid deadlock
        hook.table == DbTable::ListMetadata || hook.table == DbTable::ListMetadataState
    }

    /// Check if there are more pages to load.
    pub fn has_more_pages(&self) -> bool {
        self.list_info()
            .and_then(|info| info.total_pages.map(|total| info.current_page < total))
            .unwrap_or(true) // Unknown total or no info = assume more pages
    }

    /// Get the current page number (0 = not loaded yet).
    pub fn current_page(&self) -> u32 {
        self.list_info()
            .map(|info| info.current_page as u32)
            .unwrap_or(0)
    }

    /// Get the total number of pages, if known.
    pub fn total_pages(&self) -> Option<u32> {
        self.list_info()
            .and_then(|info| info.total_pages)
            .map(|p| p as u32)
    }

    /// Get the total number of items, if known.
    pub fn total_items(&self) -> Option<i64> {
        self.list_info().and_then(|info| info.total_items)
    }
}

// Tests for MetadataCollection are covered by integration tests in wp_mobile_integration_tests
// and by the PostMetadataCollectionWithEditContext tests which use the real database.
