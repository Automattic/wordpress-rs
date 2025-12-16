use std::sync::Arc;
use wp_api::prelude::WpGmtDateTime;
use wp_mobile_cache::{
    WpApiCache,
    db_types::db_site::DbSite,
    list_metadata::ListState,
    repository::list_metadata::{
        FetchNextPageInfo, ListMetadataHeaderUpdate, ListMetadataItemInput, ListMetadataRepository,
        RefreshInfo,
    },
};

use crate::sync::{EntityMetadata, ListMetadataReader};

use super::WpServiceError;

/// Service layer for list metadata operations.
///
/// Provides persistence for list structure (ordered entity IDs) and pagination
/// state. This replaces the in-memory `ListMetadataStore` with database-backed
/// storage that survives app restarts.
///
/// # Usage Pattern
///
/// MetadataService is typically used alongside PostService (or other entity services):
/// - PostService handles entity data (posts, with their content)
/// - MetadataService handles list metadata (which posts in what order)
///
/// # Key Responsibilities
///
/// 1. **List Structure**: Stores ordered lists of entity IDs per filter key
/// 2. **Pagination State**: Tracks current page, total pages, per_page
/// 3. **Sync State**: Tracks whether a list is idle, fetching, or errored
/// 4. **Version Control**: Enables detection of stale load-more operations
pub struct MetadataService {
    db_site: Arc<DbSite>,
    cache: Arc<WpApiCache>,
    repo: ListMetadataRepository,
}

impl MetadataService {
    /// Create a new MetadataService for a specific site.
    pub fn new(db_site: Arc<DbSite>, cache: Arc<WpApiCache>) -> Self {
        Self {
            db_site,
            cache,
            repo: ListMetadataRepository,
        }
    }

    // ============================================================
    // Read Operations
    // ============================================================

    /// Get ordered entity IDs for a list.
    ///
    /// Returns entity IDs in display order (rowid order from database).
    /// Returns empty Vec if the list doesn't exist.
    pub fn get_entity_ids(&self, key: &str) -> Result<Vec<i64>, WpServiceError> {
        self.cache.execute(|conn| {
            let items = self.repo.get_items(conn, &self.db_site, key)?;
            Ok(items.into_iter().map(|item| item.entity_id).collect())
        })
    }

    /// Get list metadata as EntityMetadata structs (for ListMetadataReader trait).
    ///
    /// Converts database items to the format expected by MetadataCollection.
    pub fn get_metadata(&self, key: &str) -> Result<Option<Vec<EntityMetadata>>, WpServiceError> {
        self.cache.execute(|conn| {
            let items = self.repo.get_items(conn, &self.db_site, key)?;

            if items.is_empty() {
                // Check if header exists - if not, list truly doesn't exist
                if self.repo.get_header(conn, &self.db_site, key)?.is_none() {
                    return Ok(None);
                }
            }

            let metadata = items
                .into_iter()
                .map(|item| {
                    let modified_gmt = item
                        .modified_gmt
                        .and_then(|s| s.parse::<WpGmtDateTime>().ok());
                    EntityMetadata::new(item.entity_id, modified_gmt, item.parent, item.menu_order)
                })
                .collect();

            Ok(Some(metadata))
        })
    }

    /// Get the current sync state for a list.
    pub fn get_state(&self, key: &str) -> Result<ListState, WpServiceError> {
        self.cache
            .execute(|conn| self.repo.get_state_by_key(conn, &self.db_site, key))
            .map_err(Into::into)
    }

    /// Get pagination info for a list.
    ///
    /// Returns None if the list doesn't exist.
    pub fn get_pagination(&self, key: &str) -> Result<Option<ListPaginationInfo>, WpServiceError> {
        self.cache.execute(|conn| {
            let header = self.repo.get_header(conn, &self.db_site, key)?;
            Ok(header.map(|h| ListPaginationInfo {
                total_pages: h.total_pages,
                total_items: h.total_items,
                current_page: h.current_page,
                per_page: h.per_page,
            }))
        })
    }

    /// Check if there are more pages to load.
    pub fn has_more_pages(&self, key: &str) -> Result<bool, WpServiceError> {
        self.cache.execute(|conn| {
            let header = match self.repo.get_header(conn, &self.db_site, key)? {
                Some(h) => h,
                None => return Ok(false),
            };

            // If we haven't loaded any pages, there are more to load
            if header.current_page == 0 {
                return Ok(true);
            }

            // If we don't know total pages, assume there might be more
            match header.total_pages {
                Some(total) => Ok(header.current_page < total),
                None => Ok(true),
            }
        })
    }

    /// Get the current version for concurrency checking.
    pub fn get_version(&self, key: &str) -> Result<i64, WpServiceError> {
        self.cache
            .execute(|conn| self.repo.get_version(conn, &self.db_site, key))
            .map_err(Into::into)
    }

    /// Check if the current version matches expected (for stale detection).
    pub fn check_version(&self, key: &str, expected_version: i64) -> Result<bool, WpServiceError> {
        self.cache
            .execute(|conn| {
                self.repo
                    .check_version(conn, &self.db_site, key, expected_version)
            })
            .map_err(Into::into)
    }

    // ============================================================
    // Write Operations
    // ============================================================

    /// Set items for a list (replaces existing items).
    ///
    /// Used for refresh (page 1) - clears existing items and stores new ones.
    /// Items are stored in the order provided.
    pub fn set_items(&self, key: &str, metadata: &[EntityMetadata]) -> Result<(), WpServiceError> {
        let items: Vec<ListMetadataItemInput> = metadata
            .iter()
            .map(|m| ListMetadataItemInput {
                entity_id: m.id,
                modified_gmt: m.modified_gmt.as_ref().map(|dt| dt.to_string()),
                parent: m.parent,
                menu_order: m.menu_order,
            })
            .collect();

        self.cache
            .execute(|conn| self.repo.set_items(conn, &self.db_site, key, &items))
            .map_err(Into::into)
    }

    /// Append items to a list (for load-more).
    ///
    /// Used for subsequent pages - adds to existing items without clearing.
    pub fn append_items(
        &self,
        key: &str,
        metadata: &[EntityMetadata],
    ) -> Result<(), WpServiceError> {
        let items: Vec<ListMetadataItemInput> = metadata
            .iter()
            .map(|m| ListMetadataItemInput {
                entity_id: m.id,
                modified_gmt: m.modified_gmt.as_ref().map(|dt| dt.to_string()),
                parent: m.parent,
                menu_order: m.menu_order,
            })
            .collect();

        self.cache
            .execute(|conn| self.repo.append_items(conn, &self.db_site, key, &items))
            .map_err(Into::into)
    }

    /// Update pagination info after a fetch.
    pub fn update_pagination(
        &self,
        key: &str,
        total_pages: Option<i64>,
        total_items: Option<i64>,
        current_page: i64,
        per_page: i64,
    ) -> Result<(), WpServiceError> {
        let update = ListMetadataHeaderUpdate {
            total_pages,
            total_items,
            current_page,
            per_page,
        };

        self.cache
            .execute(|conn| self.repo.update_header(conn, &self.db_site, key, &update))
            .map_err(Into::into)
    }

    /// Delete all data for a list.
    pub fn delete_list(&self, key: &str) -> Result<(), WpServiceError> {
        self.cache
            .execute(|conn| self.repo.delete_list(conn, &self.db_site, key))
            .map_err(Into::into)
    }

    // ============================================================
    // State Management
    // ============================================================

    /// Update sync state for a list.
    pub fn set_state(
        &self,
        key: &str,
        state: ListState,
        error_message: Option<&str>,
    ) -> Result<(), WpServiceError> {
        self.cache
            .execute(|conn| {
                self.repo
                    .update_state_by_key(conn, &self.db_site, key, state, error_message)
            })
            .map_err(Into::into)
    }

    // ============================================================
    // Concurrency Helpers
    // ============================================================

    /// Begin a refresh operation (fetch first page).
    ///
    /// This atomically:
    /// 1. Creates the list header if needed
    /// 2. Increments version (invalidates any in-flight load-more)
    /// 3. Sets state to FetchingFirstPage
    ///
    /// Returns info needed to make the API call and check version afterward.
    pub fn begin_refresh(&self, key: &str) -> Result<RefreshInfo, WpServiceError> {
        self.cache
            .execute(|conn| self.repo.begin_refresh(conn, &self.db_site, key))
            .map_err(Into::into)
    }

    /// Begin a load-next-page operation.
    ///
    /// This atomically:
    /// 1. Checks if there are more pages to load
    /// 2. Sets state to FetchingNextPage
    ///
    /// Returns None if already at last page or no pages loaded yet.
    /// Returns info including version to check before storing results.
    pub fn begin_fetch_next_page(
        &self,
        key: &str,
    ) -> Result<Option<FetchNextPageInfo>, WpServiceError> {
        self.cache
            .execute(|conn| self.repo.begin_fetch_next_page(conn, &self.db_site, key))
            .map_err(Into::into)
    }

    /// Complete a sync operation successfully.
    ///
    /// Sets state to Idle.
    pub fn complete_sync(&self, key: &str) -> Result<(), WpServiceError> {
        self.cache.execute(|conn| {
            let list_id = self.repo.get_or_create(conn, &self.db_site, key)?;
            self.repo.complete_sync(conn, list_id)
        })?;
        Ok(())
    }

    /// Complete a sync operation with error.
    ///
    /// Sets state to Error with the provided message.
    pub fn complete_sync_with_error(
        &self,
        key: &str,
        error_message: &str,
    ) -> Result<(), WpServiceError> {
        self.cache.execute(|conn| {
            let list_id = self.repo.get_or_create(conn, &self.db_site, key)?;
            self.repo
                .complete_sync_with_error(conn, list_id, error_message)
        })?;
        Ok(())
    }

    // ============================================
    // Relevance checking for update hooks
    // ============================================

    /// Get the list_metadata_id (rowid) for a given key.
    ///
    /// Returns None if no list exists for this key yet.
    /// Used by collections to cache the ID for state update matching.
    pub fn get_list_metadata_id(&self, key: &str) -> Option<i64> {
        self.cache
            .execute(|conn| self.repo.get_list_metadata_id(conn, &self.db_site, key))
            .ok()
            .flatten()
            .map(i64::from) // Convert RowId to i64 for trait interface
    }

    /// Check if a list_metadata_state row belongs to a specific list_metadata_id.
    ///
    /// Given a rowid from the list_metadata_state table (from an UpdateHook),
    /// returns true if that state row belongs to the given list_metadata_id.
    pub fn is_state_row_for_list(&self, state_row_id: i64, list_metadata_id: i64) -> bool {
        use wp_mobile_cache::RowId;

        self.cache
            .execute(|conn| {
                self.repo
                    .get_list_metadata_id_for_state_row(conn, RowId::from(state_row_id))
            })
            .ok()
            .flatten()
            .is_some_and(|id| i64::from(id) == list_metadata_id)
    }

    /// Check if a list_metadata_items row belongs to a specific key.
    ///
    /// Given a rowid from the list_metadata_items table (from an UpdateHook),
    /// returns true if that item row belongs to this service's site and the given key.
    pub fn is_item_row_for_key(&self, item_row_id: i64, key: &str) -> bool {
        use wp_mobile_cache::RowId;

        self.cache
            .execute(|conn| {
                self.repo
                    .is_item_row_for_key(conn, &self.db_site, key, RowId::from(item_row_id))
            })
            .unwrap_or(false)
    }
}

/// Implement ListMetadataReader for database-backed metadata.
///
/// This allows MetadataCollection to read list structure from the database
/// through the same trait interface it uses for in-memory stores.
///
/// Unlike the in-memory implementation, this also supports relevance checking
/// methods for split observers (data vs state updates).
impl ListMetadataReader for MetadataService {
    fn get(&self, key: &str) -> Option<Vec<EntityMetadata>> {
        self.get_metadata(key).ok().flatten()
    }

    fn get_list_metadata_id(&self, key: &str) -> Option<i64> {
        // Delegate to our existing method
        MetadataService::get_list_metadata_id(self, key)
    }

    fn is_item_row_for_key(&self, item_row_id: i64, key: &str) -> bool {
        // Delegate to our existing method
        MetadataService::is_item_row_for_key(self, item_row_id, key)
    }

    fn is_state_row_for_list(&self, state_row_id: i64, list_metadata_id: i64) -> bool {
        // Delegate to our existing method
        MetadataService::is_state_row_for_list(self, state_row_id, list_metadata_id)
    }

    fn get_sync_state(&self, key: &str) -> wp_mobile_cache::list_metadata::ListState {
        // Delegate to our existing method, default to Idle on error
        self.get_state(key).unwrap_or_default()
    }

    fn get_current_page(&self, key: &str) -> i64 {
        self.get_pagination(key)
            .ok()
            .flatten()
            .map(|p| p.current_page)
            .unwrap_or(0)
    }

    fn get_total_pages(&self, key: &str) -> Option<i64> {
        self.get_pagination(key)
            .ok()
            .flatten()
            .and_then(|p| p.total_pages)
    }
}

/// Pagination info for a list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListPaginationInfo {
    pub total_pages: Option<i64>,
    pub total_items: Option<i64>,
    pub current_page: i64,
    pub per_page: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use rusqlite::Connection;
    use wp_mobile_cache::{
        MigrationManager, WpApiCache, db_types::self_hosted_site::SelfHostedSite,
        repository::sites::SiteRepository,
    };

    struct TestContext {
        service: MetadataService,
        #[allow(dead_code)]
        cache: Arc<WpApiCache>,
    }

    #[fixture]
    fn test_ctx() -> TestContext {
        let mut conn = Connection::open_in_memory().expect("Failed to create in-memory database");
        let mut migration_manager =
            MigrationManager::new(&conn).expect("Failed to create migration manager");
        migration_manager
            .perform_migrations()
            .expect("Migrations should succeed");

        let site_repo = SiteRepository;
        let self_hosted_site = SelfHostedSite {
            url: "https://test.local".to_string(),
            api_root: "https://test.local/wp-json".to_string(),
        };
        let db_site = site_repo
            .upsert_self_hosted_site(&mut conn, &self_hosted_site)
            .expect("Site creation should succeed")
            .db_site;

        let cache = Arc::new(WpApiCache::from(conn));
        let db_site = Arc::new(db_site);
        let service = MetadataService::new(db_site, cache.clone());

        TestContext { service, cache }
    }

    #[rstest]
    fn test_get_entity_ids_returns_empty_for_non_existent(test_ctx: TestContext) {
        let ids = test_ctx.service.get_entity_ids("nonexistent").unwrap();
        assert!(ids.is_empty());
    }

    #[rstest]
    fn test_get_metadata_returns_none_for_non_existent(test_ctx: TestContext) {
        let metadata = test_ctx.service.get_metadata("nonexistent").unwrap();
        assert!(metadata.is_none());
    }

    #[rstest]
    fn test_set_and_get_items(test_ctx: TestContext) {
        let key = "edit:posts:publish";
        let metadata = vec![
            EntityMetadata::new(100, None, None, None),
            EntityMetadata::new(200, None, None, None),
            EntityMetadata::new(300, None, None, None),
        ];

        test_ctx.service.set_items(key, &metadata).unwrap();

        let ids = test_ctx.service.get_entity_ids(key).unwrap();
        assert_eq!(ids, vec![100, 200, 300]);
    }

    #[rstest]
    fn test_set_items_replaces_existing(test_ctx: TestContext) {
        let key = "edit:posts:draft";

        test_ctx
            .service
            .set_items(
                key,
                &[
                    EntityMetadata::new(1, None, None, None),
                    EntityMetadata::new(2, None, None, None),
                ],
            )
            .unwrap();

        test_ctx
            .service
            .set_items(
                key,
                &[
                    EntityMetadata::new(10, None, None, None),
                    EntityMetadata::new(20, None, None, None),
                ],
            )
            .unwrap();

        let ids = test_ctx.service.get_entity_ids(key).unwrap();
        assert_eq!(ids, vec![10, 20]);
    }

    #[rstest]
    fn test_append_items(test_ctx: TestContext) {
        let key = "edit:posts:pending";

        test_ctx
            .service
            .set_items(key, &[EntityMetadata::new(1, None, None, None)])
            .unwrap();

        test_ctx
            .service
            .append_items(
                key,
                &[
                    EntityMetadata::new(2, None, None, None),
                    EntityMetadata::new(3, None, None, None),
                ],
            )
            .unwrap();

        let ids = test_ctx.service.get_entity_ids(key).unwrap();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[rstest]
    fn test_get_state_returns_idle_for_non_existent(test_ctx: TestContext) {
        let state = test_ctx.service.get_state("nonexistent").unwrap();
        assert_eq!(state, ListState::Idle);
    }

    #[rstest]
    fn test_set_and_get_state(test_ctx: TestContext) {
        let key = "edit:posts:publish";

        test_ctx
            .service
            .set_state(key, ListState::FetchingFirstPage, None)
            .unwrap();

        let state = test_ctx.service.get_state(key).unwrap();
        assert_eq!(state, ListState::FetchingFirstPage);
    }

    #[rstest]
    fn test_update_and_get_pagination(test_ctx: TestContext) {
        let key = "edit:posts:publish";

        test_ctx
            .service
            .update_pagination(key, Some(5), Some(100), 1, 20)
            .unwrap();

        let pagination = test_ctx.service.get_pagination(key).unwrap().unwrap();
        assert_eq!(pagination.total_pages, Some(5));
        assert_eq!(pagination.total_items, Some(100));
        assert_eq!(pagination.current_page, 1);
        assert_eq!(pagination.per_page, 20);
    }

    #[rstest]
    fn test_has_more_pages(test_ctx: TestContext) {
        let key = "edit:posts:publish";

        // No pages loaded yet
        test_ctx
            .service
            .update_pagination(key, Some(3), None, 0, 20)
            .unwrap();
        assert!(test_ctx.service.has_more_pages(key).unwrap());

        // Page 1 of 3 loaded
        test_ctx
            .service
            .update_pagination(key, Some(3), None, 1, 20)
            .unwrap();
        assert!(test_ctx.service.has_more_pages(key).unwrap());

        // Page 3 of 3 loaded (no more)
        test_ctx
            .service
            .update_pagination(key, Some(3), None, 3, 20)
            .unwrap();
        assert!(!test_ctx.service.has_more_pages(key).unwrap());
    }

    #[rstest]
    fn test_begin_refresh_increments_version(test_ctx: TestContext) {
        let key = "edit:posts:publish";

        let info1 = test_ctx.service.begin_refresh(key).unwrap();
        assert_eq!(info1.version, 1);

        test_ctx.service.complete_sync(key).unwrap();

        let info2 = test_ctx.service.begin_refresh(key).unwrap();
        assert_eq!(info2.version, 2);
    }

    #[rstest]
    fn test_begin_fetch_next_page_returns_none_when_no_pages(test_ctx: TestContext) {
        let key = "edit:posts:publish";

        // Create header but don't load any pages
        test_ctx.service.begin_refresh(key).unwrap();
        test_ctx.service.complete_sync(key).unwrap();

        let result = test_ctx.service.begin_fetch_next_page(key).unwrap();
        assert!(result.is_none());
    }

    #[rstest]
    fn test_begin_fetch_next_page_returns_info_when_more_pages(test_ctx: TestContext) {
        let key = "edit:posts:publish";

        // Set up: page 1 of 3 loaded
        test_ctx.service.begin_refresh(key).unwrap();
        test_ctx
            .service
            .update_pagination(key, Some(3), None, 1, 20)
            .unwrap();
        test_ctx.service.complete_sync(key).unwrap();

        let result = test_ctx.service.begin_fetch_next_page(key).unwrap();
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.page, 2);
    }

    #[rstest]
    fn test_delete_list(test_ctx: TestContext) {
        let key = "edit:posts:publish";

        test_ctx
            .service
            .set_items(key, &[EntityMetadata::new(1, None, None, None)])
            .unwrap();
        test_ctx
            .service
            .update_pagination(key, Some(1), None, 1, 20)
            .unwrap();

        test_ctx.service.delete_list(key).unwrap();

        assert!(test_ctx.service.get_metadata(key).unwrap().is_none());
        assert!(test_ctx.service.get_pagination(key).unwrap().is_none());
    }

    #[rstest]
    fn test_list_metadata_reader_trait(test_ctx: TestContext) {
        let key = "edit:posts:publish";
        let metadata = vec![
            EntityMetadata::new(100, None, None, None),
            EntityMetadata::new(200, None, None, None),
        ];

        test_ctx.service.set_items(key, &metadata).unwrap();

        // Access via trait
        let reader: &dyn ListMetadataReader = &test_ctx.service;
        let result = reader.get(key).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, 100);
        assert_eq!(result[1].id, 200);
    }

    #[rstest]
    fn test_list_metadata_reader_returns_none_for_non_existent(test_ctx: TestContext) {
        let reader: &dyn ListMetadataReader = &test_ctx.service;
        assert!(reader.get("nonexistent").is_none());
    }
}
