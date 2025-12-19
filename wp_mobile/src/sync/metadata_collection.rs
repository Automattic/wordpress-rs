use std::sync::Arc;

use wp_mobile_cache::{DbTable, UpdateHook, list_metadata::ListKey};

use crate::collection::FetchError;

use super::{
    CollectionItem, EntityStateReader, ListInfo, ListMetadataReader, MetadataFetcher, SyncResult,
};

/// Collection that uses metadata-first fetching strategy.
///
/// This collection type:
/// 1. Uses lightweight metadata (id + modified_gmt) to define list structure
/// 2. Shows cached entities immediately via `CollectionItem` states
/// 3. Tracks which entities are missing or stale for selective fetching
///
/// # Type Parameter
/// - `F`: The fetcher implementation (e.g., `PostMetadataFetcher`)
///
/// # Usage Flow
/// 1. Create collection with filter-specific fetcher
/// 2. Call `refresh()` to fetch metadata and sync missing entities
/// 3. Call `items()` to get current list with states
/// 4. Call `load_next_page()` for pagination
/// 5. Use `is_relevant_update()` to check if DB changes affect this collection
///
/// # Example
/// ```ignore
/// let fetcher = PostMetadataFetcher::new(&service, filter, kv_key);
/// let mut collection = MetadataCollection::new(
///     kv_key,
///     service.metadata_reader(),
///     service.state_reader(),
///     fetcher,
///     vec![DbTable::PostsEditContext],
/// );
///
/// // Initial load
/// collection.refresh().await?;
///
/// // Get items with states
/// let items = collection.items();
/// for item in items {
///     match item.state {
///         EntityState::Cached => { /* show full entity */ }
///         EntityState::Fetching => { /* show loading */ }
///         EntityState::Failed { .. } => { /* show error */ }
///         _ => { /* show placeholder */ }
///     }
/// }
/// ```
pub struct MetadataCollection<F>
where
    F: MetadataFetcher,
{
    /// Key for metadata store lookup
    key: ListKey,

    /// Read-only access to list metadata
    metadata_reader: Arc<dyn ListMetadataReader>,

    /// Read-only access to entity states
    state_reader: Arc<dyn EntityStateReader>,

    /// Fetcher for metadata and full entities
    fetcher: F,

    /// Tables to monitor for data updates (entity tables like PostsEditContext)
    relevant_data_tables: Vec<DbTable>,

    /// Items per page configuration (default: 20)
    per_page: u32,
}

impl<F> MetadataCollection<F>
where
    F: MetadataFetcher,
{
    /// Create a new metadata collection.
    ///
    /// # Arguments
    /// * `key` - Key for metadata store lookup (e.g., "site_1:posts:publish")
    /// * `metadata_reader` - Read-only access to list metadata store
    /// * `state_reader` - Read-only access to entity state store
    /// * `fetcher` - Implementation for fetching metadata and entities
    /// * `relevant_data_tables` - DB tables to monitor for data updates (entity tables)
    pub fn new(
        key: ListKey,
        metadata_reader: Arc<dyn ListMetadataReader>,
        state_reader: Arc<dyn EntityStateReader>,
        fetcher: F,
        relevant_data_tables: Vec<DbTable>,
    ) -> Self {
        Self {
            key,
            metadata_reader,
            state_reader,
            fetcher,
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

    /// Refresh the collection (fetch page 1, replace metadata).
    ///
    /// This:
    /// 1. Fetches metadata from the network (page 1)
    /// 2. Replaces existing metadata in the store
    /// 3. Fetches missing/stale entities
    ///
    /// Returns sync statistics including counts and pagination info.
    pub async fn refresh(&self) -> Result<SyncResult, FetchError> {
        println!("[MetadataCollection] Refreshing collection...");

        let result = self.fetcher.fetch_metadata(1, self.per_page, true).await?;

        let total_pages_str = result
            .total_pages
            .map(|p| p.to_string())
            .unwrap_or_else(|| "?".to_string());
        println!(
            "[MetadataCollection] Fetched metadata: page 1 of {}, {} items",
            total_pages_str,
            result.metadata.len()
        );

        self.sync_missing_and_stale().await
    }

    /// Load the next page of items.
    ///
    /// This:
    /// 1. Fetches metadata for the next page
    /// 2. Appends to existing metadata in the store
    /// 3. Fetches missing/stale entities from the new page
    ///
    /// Returns `SyncResult::no_op()` if already on the last page or no pages loaded yet.
    pub async fn load_next_page(&self) -> Result<SyncResult, FetchError> {
        let current_page = self.current_page();
        let total_pages = self.total_pages();

        // Check if no pages have been loaded yet (need refresh first)
        if current_page == 0 {
            println!("[MetadataCollection] No pages loaded yet, need refresh first");
            return Ok(SyncResult::no_op(
                self.items().len(),
                true, // has_more_pages = true, but need refresh first
                0,
                None,
            ));
        }

        let next_page = current_page + 1;

        // Check if we're already at the last page
        if total_pages.is_some_and(|total| next_page > total) {
            println!("[MetadataCollection] Already at last page, nothing to load");
            return Ok(SyncResult::no_op(
                self.items().len(),
                false,
                current_page,
                total_pages,
            ));
        }

        println!("[MetadataCollection] Loading page {}...", next_page);

        let result = self
            .fetcher
            .fetch_metadata(next_page, self.per_page, false)
            .await?;

        let total_pages_str = result
            .total_pages
            .map(|p| p.to_string())
            .unwrap_or_else(|| "?".to_string());
        println!(
            "[MetadataCollection] Fetched metadata: page {} of {}, {} items",
            next_page,
            total_pages_str,
            result.metadata.len()
        );

        self.sync_missing_and_stale().await
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

    /// Fetch missing and stale items.
    async fn sync_missing_and_stale(&self) -> Result<SyncResult, FetchError> {
        use super::EntityState;

        let items = self.items();
        let total_items = items.len();

        // Count by state for logging
        let missing_count = items
            .iter()
            .filter(|item| matches!(item.state, EntityState::Missing))
            .count();
        let stale_count = items
            .iter()
            .filter(|item| matches!(item.state, EntityState::Stale))
            .count();
        let cached_count = items
            .iter()
            .filter(|item| matches!(item.state, EntityState::Cached))
            .count();

        // Collect IDs that need fetching
        let ids_to_fetch: Vec<i64> = items
            .iter()
            .filter(|item| item.needs_fetch())
            .map(|item| item.id())
            .collect();

        let fetch_count = ids_to_fetch.len();

        println!(
            "[MetadataCollection] Sync: {} items total ({} cached, {} missing, {} stale)",
            total_items, cached_count, missing_count, stale_count
        );

        if !ids_to_fetch.is_empty() {
            println!(
                "[MetadataCollection] Fetching {} posts by ID...",
                fetch_count
            );

            // Batch into chunks of 100 (WordPress API limit)
            for chunk in ids_to_fetch.chunks(100) {
                self.fetcher.ensure_fetched(chunk.to_vec()).await?;
            }
        } else {
            println!("[MetadataCollection] All items already cached, nothing to fetch");
        }

        // Count failures after fetch attempts
        let failed_count = self
            .items()
            .iter()
            .filter(|item| item.state.is_failed())
            .count();

        if fetch_count > 0 {
            let success_count = fetch_count - failed_count;
            println!(
                "[MetadataCollection] Fetched {} posts ({} succeeded, {} failed)",
                fetch_count, success_count, failed_count
            );
        }

        Ok(SyncResult::new(
            total_items,
            fetch_count,
            failed_count,
            self.has_more_pages(),
            self.current_page(),
            self.total_pages(),
        ))
    }
}

// Tests for MetadataCollection are covered by integration tests in wp_mobile_integration_tests
// and by the PostMetadataCollectionWithEditContext tests which use the real database.
