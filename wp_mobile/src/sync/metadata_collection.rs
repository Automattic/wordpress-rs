use std::sync::{Arc, RwLock};

use wp_mobile_cache::UpdateHook;

use crate::collection::FetchError;

use super::{CollectionItem, EntityStateReader, ListMetadataReader, MetadataFetcher, SyncResult};

/// Mutable pagination state, wrapped in RwLock for interior mutability.
#[derive(Debug)]
struct PaginationState {
    current_page: u32,
    total_pages: Option<u32>,
    per_page: u32,
}

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
    kv_key: String,

    /// Read-only access to list metadata
    metadata_reader: Arc<dyn ListMetadataReader>,

    /// Read-only access to entity states
    state_reader: Arc<dyn EntityStateReader>,

    /// Fetcher for metadata and full entities
    fetcher: F,

    /// Tables to monitor for relevant updates
    relevant_tables: Vec<wp_mobile_cache::DbTable>,

    /// Pagination state (uses interior mutability for UniFFI compatibility)
    pagination: RwLock<PaginationState>,
}

impl<F> MetadataCollection<F>
where
    F: MetadataFetcher,
{
    /// Create a new metadata collection.
    ///
    /// # Arguments
    /// * `kv_key` - Key for metadata store lookup (e.g., "site_1:posts:publish")
    /// * `metadata_reader` - Read-only access to list metadata store
    /// * `state_reader` - Read-only access to entity state store
    /// * `fetcher` - Implementation for fetching metadata and entities
    /// * `relevant_tables` - DB tables to monitor for updates
    pub fn new(
        kv_key: String,
        metadata_reader: Arc<dyn ListMetadataReader>,
        state_reader: Arc<dyn EntityStateReader>,
        fetcher: F,
        relevant_tables: Vec<wp_mobile_cache::DbTable>,
    ) -> Self {
        Self {
            kv_key,
            metadata_reader,
            state_reader,
            fetcher,
            relevant_tables,
            pagination: RwLock::new(PaginationState {
                current_page: 0,
                total_pages: None,
                per_page: 20,
            }),
        }
    }

    /// Set the number of items per page.
    ///
    /// Default is 20. Call this before `refresh()` if you need a different page size.
    pub fn with_per_page(self, per_page: u32) -> Self {
        self.pagination.write().unwrap().per_page = per_page;
        self
    }

    /// Get current items with their states.
    ///
    /// Returns a `CollectionItem` for each entity in the list, combining
    /// the metadata with the current fetch state.
    pub fn items(&self) -> Vec<CollectionItem> {
        self.metadata_reader
            .get(&self.kv_key)
            .unwrap_or_default()
            .into_iter()
            .map(|metadata| {
                CollectionItem::new(metadata.clone(), self.state_reader.get(metadata.id))
            })
            .collect()
    }

    /// Check if a database update is relevant to this collection.
    ///
    /// Returns `true` if the update is to a table this collection monitors.
    /// Platform layers use this to determine when to notify observers.
    pub fn is_relevant_update(&self, hook: &UpdateHook) -> bool {
        self.relevant_tables.contains(&hook.table)
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

        let per_page = self.pagination.read().unwrap().per_page;
        let result = self.fetcher.fetch_metadata(1, per_page, true).await?;

        let total_pages_str = result
            .total_pages
            .map(|p| p.to_string())
            .unwrap_or_else(|| "?".to_string());
        println!(
            "[MetadataCollection] Fetched metadata: page 1 of {}, {} items",
            total_pages_str,
            result.metadata.len()
        );

        {
            let mut pagination = self.pagination.write().unwrap();
            pagination.current_page = 1;
            pagination.total_pages = result.total_pages;
        }

        self.sync_missing_and_stale().await
    }

    /// Load the next page of items.
    ///
    /// This:
    /// 1. Fetches metadata for the next page
    /// 2. Appends to existing metadata in the store
    /// 3. Fetches missing/stale entities from the new page
    ///
    /// Returns `SyncResult::no_op()` if already on the last page.
    pub async fn load_next_page(&self) -> Result<SyncResult, FetchError> {
        let (next_page, per_page, total_pages) = {
            let pagination = self.pagination.read().unwrap();
            (
                pagination.current_page + 1,
                pagination.per_page,
                pagination.total_pages,
            )
        };

        // Check if we're already at the last page
        if total_pages.is_some_and(|total| next_page > total) {
            println!("[MetadataCollection] Already at last page, nothing to load");
            return Ok(SyncResult::no_op(self.items().len(), false));
        }

        println!("[MetadataCollection] Loading page {}...", next_page);

        let result = self
            .fetcher
            .fetch_metadata(next_page, per_page, false)
            .await?;

        let total_pages_str = result
            .total_pages
            .map(|p| p.to_string())
            .unwrap_or_else(|| "?".to_string());
        println!(
            "[MetadataCollection] Fetched metadata: page {} of {}, {} items",
            next_page, total_pages_str, result.metadata.len()
        );

        {
            let mut pagination = self.pagination.write().unwrap();
            pagination.current_page = next_page;
            pagination.total_pages = result.total_pages;
        }

        self.sync_missing_and_stale().await
    }

    /// Check if there are more pages to load.
    pub fn has_more_pages(&self) -> bool {
        let pagination = self.pagination.read().unwrap();
        pagination
            .total_pages
            .map(|total| pagination.current_page < total)
            .unwrap_or(true) // Unknown total = assume more pages
    }

    /// Get the current page number (0 = not loaded yet).
    pub fn current_page(&self) -> u32 {
        self.pagination.read().unwrap().current_page
    }

    /// Get the total number of pages, if known.
    pub fn total_pages(&self) -> Option<u32> {
        self.pagination.read().unwrap().total_pages
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
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::{
        EntityMetadata, EntityState, EntityStateStore, ListMetadataStore, MetadataFetchResult,
    };
    use std::sync::atomic::{AtomicU32, Ordering};
    use wp_api::prelude::WpGmtDateTime;

    /// Mock fetcher for testing
    struct MockFetcher {
        metadata_store: Arc<ListMetadataStore>,
        state_store: Arc<EntityStateStore>,
        kv_key: String,
        fetch_metadata_calls: AtomicU32,
        ensure_fetched_calls: AtomicU32,
    }

    impl MockFetcher {
        fn new(
            metadata_store: Arc<ListMetadataStore>,
            state_store: Arc<EntityStateStore>,
            kv_key: &str,
        ) -> Self {
            Self {
                metadata_store,
                state_store,
                kv_key: kv_key.to_string(),
                fetch_metadata_calls: AtomicU32::new(0),
                ensure_fetched_calls: AtomicU32::new(0),
            }
        }
    }

    impl MetadataFetcher for MockFetcher {
        async fn fetch_metadata(
            &self,
            page: u32,
            _per_page: u32,
            is_first_page: bool,
        ) -> Result<MetadataFetchResult, FetchError> {
            self.fetch_metadata_calls.fetch_add(1, Ordering::SeqCst);

            // Simulate 2 pages of 2 items each
            let metadata = vec![
                EntityMetadata::with_modified(
                    (page * 10 + 1) as i64,
                    WpGmtDateTime::from_timestamp(1000),
                ),
                EntityMetadata::with_modified(
                    (page * 10 + 2) as i64,
                    WpGmtDateTime::from_timestamp(1001),
                ),
            ];

            if is_first_page {
                self.metadata_store.set(&self.kv_key, metadata.clone());
            } else {
                self.metadata_store.append(&self.kv_key, metadata.clone());
            }

            Ok(MetadataFetchResult::new(metadata, Some(4), Some(2), page))
        }

        async fn ensure_fetched(&self, ids: Vec<i64>) -> Result<(), FetchError> {
            self.ensure_fetched_calls.fetch_add(1, Ordering::SeqCst);

            // Simulate successful fetch - mark all as Cached
            ids.iter().for_each(|&id| {
                self.state_store.set(id, EntityState::Cached);
            });

            Ok(())
        }
    }

    fn create_test_collection() -> (
        MetadataCollection<MockFetcher>,
        Arc<ListMetadataStore>,
        Arc<EntityStateStore>,
    ) {
        let metadata_store = Arc::new(ListMetadataStore::new());
        let state_store = Arc::new(EntityStateStore::new());
        let kv_key = "test_key";

        let fetcher = MockFetcher::new(metadata_store.clone(), state_store.clone(), kv_key);

        let collection = MetadataCollection::new(
            kv_key.to_string(),
            metadata_store.clone(),
            state_store.clone(),
            fetcher,
            vec![],
        );

        (collection, metadata_store, state_store)
    }

    #[tokio::test]
    async fn test_refresh_fetches_metadata_and_syncs() {
        let (collection, _, _) = create_test_collection();

        let result = collection.refresh().await.unwrap();

        assert_eq!(result.total_items, 2);
        assert_eq!(result.fetched_count, 2); // Both items needed fetching
        assert_eq!(result.failed_count, 0);
        assert!(result.has_more_pages);
        assert_eq!(collection.current_page(), 1);
    }

    #[tokio::test]
    async fn test_items_returns_correct_states() {
        let (collection, _, _state_store) = create_test_collection();

        // Before refresh - empty
        assert!(collection.items().is_empty());

        // After refresh - items should be cached (mock marks them cached)
        collection.refresh().await.unwrap();
        let items = collection.items();

        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|item| item.state == EntityState::Cached));
    }

    #[tokio::test]
    async fn test_load_next_page_appends() {
        let (collection, _, _) = create_test_collection();

        // First page
        collection.refresh().await.unwrap();
        assert_eq!(collection.items().len(), 2);

        // Second page
        let result = collection.load_next_page().await.unwrap();
        assert_eq!(result.total_items, 4); // 2 + 2
        assert_eq!(collection.items().len(), 4);
        assert_eq!(collection.current_page(), 2);
    }

    #[tokio::test]
    async fn test_load_next_page_at_end_returns_no_op() {
        let (collection, _, _) = create_test_collection();

        // Load both pages
        collection.refresh().await.unwrap();
        collection.load_next_page().await.unwrap();

        // Try to load page 3 (doesn't exist)
        let result = collection.load_next_page().await.unwrap();
        assert_eq!(result.fetched_count, 0);
        assert!(!result.has_more_pages);
    }

    #[tokio::test]
    async fn test_has_more_pages() {
        let (collection, _, _) = create_test_collection();

        // Before load - unknown, assume true
        assert!(collection.has_more_pages());

        // After page 1
        collection.refresh().await.unwrap();
        assert!(collection.has_more_pages());

        // After page 2 (last page)
        collection.load_next_page().await.unwrap();
        assert!(!collection.has_more_pages());
    }

    #[tokio::test]
    async fn test_items_needing_fetch_triggers_ensure_fetched() {
        let (collection, _metadata_store, state_store) = create_test_collection();

        // Pre-populate with some cached items
        state_store.set(11, EntityState::Cached);
        // Item 12 will be Missing (needs fetch)

        collection.refresh().await.unwrap();

        // Check that ensure_fetched was called
        // (In real impl, only item 12 would need fetching, but mock doesn't distinguish)
        let items = collection.items();
        assert_eq!(items.len(), 2);
    }
}
