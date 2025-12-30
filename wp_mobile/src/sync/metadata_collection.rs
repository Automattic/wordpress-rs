use std::sync::Arc;
use wp_mobile_cache::{
    DbTable, UpdateHook,
    list_metadata::{ListKey, ListState},
};

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

    /// Items per page configuration
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
    /// * `per_page` - Number of items per page
    pub fn new(
        key: ListKey,
        metadata_reader: Arc<dyn ListMetadataReader>,
        state_reader: Arc<dyn EntityStateReader>,
        relevant_data_tables: Vec<DbTable>,
        per_page: u32,
    ) -> Self {
        Self {
            key,
            metadata_reader,
            state_reader,
            relevant_data_tables,
            per_page,
        }
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
    ///
    /// Returns:
    /// - `None` - List hasn't been loaded yet (no metadata stored)
    /// - `Some(vec![])` - List was loaded but is empty
    /// - `Some(vec![...])` - List has items
    pub fn items(&self) -> Option<Vec<CollectionItem>> {
        self.metadata_reader.get_items(&self.key).map(|items| {
            items
                .into_iter()
                .map(|metadata| {
                    let id = metadata.id;
                    CollectionItem::new(metadata, self.state_reader.get(id))
                })
                .collect()
        })
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
    /// - `Idle` - No sync in progress (or no metadata loaded yet)
    /// - `FetchingFirstPage` - Refresh in progress
    /// - `FetchingNextPage` - Load more in progress
    /// - `Error` - Last sync failed
    ///
    /// Use this to show loading indicators in the UI.
    pub fn sync_state(&self) -> ListState {
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
    /// - The ListMetadataItems table (any row)
    ///
    /// Use this for data observers that should refresh list contents.
    ///
    /// Note: This check is intentionally kept simple and table-based only. We don't
    /// query the database to filter by key because hook callbacks must be fast and
    /// can't safely access the DB during a transaction. This means we may get false
    /// positives for updates from other collections, but that's safe (just extra refreshes).
    pub fn is_relevant_data_update(&self, hook: &UpdateHook) -> bool {
        // Check entity tables
        if self.relevant_data_tables.contains(&hook.table) {
            return true;
        }

        // Check ListMetadataItems - accept any update to keep this check simple and fast
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
    /// Note: This check is intentionally kept simple and table-based only. We don't
    /// query the database to filter by key because hook callbacks must be fast and
    /// can't safely access the DB during a transaction. This means we may get false
    /// positives for updates from other collections, but that's safe (just extra reads).
    pub fn is_relevant_list_info_update(&self, hook: &UpdateHook) -> bool {
        hook.table == DbTable::ListMetadata || hook.table == DbTable::ListMetadataState
    }

    /// Check if there are more pages to load.
    ///
    /// Returns:
    /// - `None` - Unknown (no metadata loaded or total_pages not provided by API)
    /// - `Some(true)` - More pages available
    /// - `Some(false)` - On last page
    pub fn has_more_pages(&self) -> Option<bool> {
        self.list_info().and_then(|info| {
            info.current_page
                .and_then(|current| info.total_pages.map(|total| current < total))
        })
    }

    /// Get the current page number.
    ///
    /// Returns:
    /// - `None` - No metadata loaded yet
    /// - `Some(n)` - Currently on page n
    pub fn current_page(&self) -> Option<u32> {
        self.list_info().and_then(|info| info.current_page)
    }

    /// Get the total number of pages, if known.
    pub fn total_pages(&self) -> Option<u32> {
        self.list_info().and_then(|info| info.total_pages)
    }

    /// Get the total number of items, if known.
    pub fn total_items(&self) -> Option<i64> {
        self.list_info().and_then(|info| info.total_items)
    }

    /// Load the next page using a closure-based orchestrator pattern.
    ///
    /// This method handles all common pagination logic:
    /// 1. Checks if any pages have been loaded (returns no-op if refresh needed)
    /// 2. Checks if already at last page (returns no-op if done)
    /// 3. Delegates to the provided fetch function if checks pass
    ///
    /// The fetch function should perform the actual network request and return
    /// a `SyncResult`. The core orchestrates the pagination flow.
    ///
    /// # Arguments
    /// * `fetch_fn` - Async closure that performs the fetch operation
    ///
    /// # Returns
    /// * `Ok(SyncResult::no_op(...))` - If no fetch needed (not refreshed or on last page)
    /// * `Ok(SyncResult)` - Result from the fetch operation
    /// * `Err(E)` - Error from the fetch operation
    ///
    /// # Example
    /// ```ignore
    /// let result = core.load_next_page_with(|| async {
    ///     service.sync_list(core.key(), &endpoint, &filter, core.per_page(), false).await
    /// }).await?;
    /// ```
    pub async fn load_next_page_with<F, Fut, E>(&self, fetch_fn: F) -> Result<super::SyncResult, E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<super::SyncResult, E>>,
    {
        let current_page = self.current_page();
        let total_pages = self.total_pages();

        // Check if no pages have been loaded yet (need refresh first)
        let Some(current_page) = current_page else {
            log::debug!("MetadataCollection: No pages loaded yet, need refresh first");
            return Ok(super::SyncResult::no_op(
                self.items().map(|items| items.len()).unwrap_or(0),
                Some(true), // has_more_pages = true, but need refresh first
                None,       // current_page = None (not loaded)
                None,       // total_pages = None
            ));
        };

        // Check if we're already at the last page (early exit for UX)
        if total_pages.is_some_and(|total| current_page >= total) {
            log::debug!("MetadataCollection: Already at last page, nothing to load");
            return Ok(super::SyncResult::no_op(
                self.items().map(|items| items.len()).unwrap_or(0),
                Some(false), // has_more_pages = false (on last page)
                Some(current_page),
                total_pages,
            ));
        }

        log::debug!("MetadataCollection: Loading next page");

        // All checks passed, delegate to the fetch function
        fetch_fn().await
    }
}
