use std::{future::Future, sync::Arc};
use wp_api::prelude::WpGmtDateTime;
use wp_mobile_cache::{
    RowId, WpApiCache,
    db_types::db_site::DbSite,
    list_metadata::{ListKey, ListState},
    repository::list_metadata::{
        ListMetadataHeaderUpdate, ListMetadataItemInput, ListMetadataRepository,
    },
};

use crate::{
    collection::FetchError,
    sync::{EntityMetadata, ListInfo, ListMetadataReader, MetadataSyncManager, SyncSession},
};

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
}

impl MetadataService {
    /// Create a new MetadataService for a specific site.
    pub fn new(db_site: Arc<DbSite>, cache: Arc<WpApiCache>) -> Self {
        Self { db_site, cache }
    }

    // ============================================================
    // Read Operations
    // ============================================================

    /// Get ordered entity IDs for a list.
    ///
    /// Returns entity IDs in display order (rowid order from database).
    /// Returns empty Vec if the list doesn't exist.
    pub fn get_entity_ids(&self, key: &ListKey) -> Result<Vec<i64>, WpServiceError> {
        self.cache.execute(|conn| {
            let items = ListMetadataRepository::get_items_by_list_key(conn, &self.db_site, key)?;
            Ok(items.into_iter().map(|item| item.entity_id).collect())
        })
    }

    /// Get list metadata as EntityMetadata structs (for ListMetadataReader trait).
    ///
    /// Converts database items to the format expected by MetadataCollection.
    pub fn get_metadata(
        &self,
        key: &ListKey,
    ) -> Result<Option<Vec<EntityMetadata>>, WpServiceError> {
        self.cache.execute(|conn| {
            let items = ListMetadataRepository::get_items_by_list_key(conn, &self.db_site, key)?;

            if items.is_empty() {
                // Check if header exists - if not, list truly doesn't exist
                if ListMetadataRepository::get_header(conn, &self.db_site, key)?.is_none() {
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
    pub fn get_state(&self, key: &ListKey) -> Result<ListState, WpServiceError> {
        self.cache
            .execute(|conn| ListMetadataRepository::get_state_by_list_key(conn, &self.db_site, key))
            .map_err(Into::into)
    }

    /// Get pagination info for a list.
    ///
    /// Returns None if the list doesn't exist.
    pub fn get_pagination(
        &self,
        key: &ListKey,
    ) -> Result<Option<ListPaginationInfo>, WpServiceError> {
        self.cache.execute(|conn| {
            let header = ListMetadataRepository::get_header(conn, &self.db_site, key)?;
            Ok(header.map(|h| ListPaginationInfo {
                total_pages: h.total_pages,
                total_items: h.total_items,
                current_page: h.current_page,
                per_page: h.per_page,
            }))
        })
    }

    /// Check if there are more pages to load.
    pub fn has_more_pages(&self, key: &ListKey) -> Result<bool, WpServiceError> {
        self.cache.execute(|conn| {
            let header = match ListMetadataRepository::get_header(conn, &self.db_site, key)? {
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
    pub fn get_version(&self, key: &ListKey) -> Result<i64, WpServiceError> {
        self.cache
            .execute(|conn| ListMetadataRepository::get_version(conn, &self.db_site, key))
            .map_err(Into::into)
    }

    /// Check if the current version matches expected (for stale detection).
    pub fn check_version(
        &self,
        key: &ListKey,
        expected_version: i64,
    ) -> Result<bool, WpServiceError> {
        let current_version = self.get_version(key)?;
        Ok(current_version == expected_version)
    }

    // ============================================================
    // Write Operations
    // ============================================================

    /// Set items for a list (replaces existing items).
    ///
    /// Used for refresh (page 1) - clears existing items and stores new ones.
    /// Items are stored in the order provided.
    pub fn set_items(
        &self,
        key: &ListKey,
        per_page: i64,
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
            .execute(|conn| {
                ListMetadataRepository::set_items_by_list_key(
                    conn,
                    &self.db_site,
                    key,
                    per_page,
                    &items,
                )
            })
            .map_err(Into::into)
    }

    /// Append items to a list (for load-more).
    ///
    /// Used for subsequent pages - adds to existing items without clearing.
    pub fn append_items(
        &self,
        key: &ListKey,
        per_page: i64,
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
            .execute(|conn| {
                ListMetadataRepository::append_items_by_list_key(
                    conn,
                    &self.db_site,
                    key,
                    per_page,
                    &items,
                )
            })
            .map_err(Into::into)
    }

    /// Update pagination info after a fetch.
    pub fn update_pagination(
        &self,
        key: &ListKey,
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
            .execute(|conn| {
                ListMetadataRepository::update_header_by_list_key(conn, &self.db_site, key, &update)
            })
            .map_err(Into::into)
    }

    /// Delete all data for a list.
    pub fn delete_list(&self, key: &ListKey) -> Result<(), WpServiceError> {
        self.cache
            .execute(|conn| ListMetadataRepository::delete_list(conn, &self.db_site, key))
            .map_err(Into::into)
    }

    // ============================================================
    // State Management
    // ============================================================

    /// Update sync state for a list.
    pub fn set_state(
        &self,
        key: &ListKey,
        per_page: i64,
        state: ListState,
        error_message: Option<&str>,
    ) -> Result<(), WpServiceError> {
        self.cache
            .execute(|conn| {
                ListMetadataRepository::update_state_by_list_key(
                    conn,
                    &self.db_site,
                    key,
                    per_page,
                    state,
                    error_message,
                )
            })
            .map_err(Into::into)
    }

    // ============================================================
    // Orchestration API (async, owns lifecycle)
    // ============================================================

    /// Refresh a list by fetching the first page and replacing existing data.
    ///
    /// Orchestrates the full sync lifecycle:
    /// 1. Increment version (invalidates in-flight load-more)
    /// 2. Set state to FetchingFirstPage
    /// 3. Call the fetcher with (page=1, per_page)
    /// 4. Store metadata (replacing existing)
    /// 5. Update pagination
    /// 6. Set state to Idle (or Error on failure)
    ///
    /// # Arguments
    /// * `key` - The list key identifying which list to refresh
    /// * `per_page` - Items per page for the fetch
    /// * `fetcher` - Async closure that fetches metadata, receives (page, per_page)
    ///
    /// # Returns
    /// - `Ok(MetadataFetchResult)` on success
    /// - `Err(FetchError)` on failure (state is set to Error)
    ///
    /// # Example
    /// ```ignore
    /// let result = metadata_service.refresh(
    ///     &key,
    ///     25,
    ///     |page, per_page| async move {
    ///         api_client.fetch_metadata(page, per_page).await
    ///     },
    /// ).await?;
    /// ```
    pub async fn refresh<F, Fut>(
        &self,
        key: &ListKey,
        per_page: u32,
        fetcher: F,
    ) -> Result<crate::sync::MetadataFetchResult, FetchError>
    where
        F: FnOnce(u32, u32) -> Fut,
        Fut: Future<Output = Result<crate::sync::MetadataFetchResult, FetchError>>,
    {
        log::debug!("MetadataService::refresh: key={}, per_page={}", key, per_page);

        // 1. Begin refresh (increment version, set state to FetchingFirstPage)
        let info = self.cache.execute(|conn| {
            MetadataSyncManager::begin_refresh(conn, &self.db_site, key, per_page as i64)
        })?;

        // 2. Call fetcher with page=1 (if this fails, set error state)
        let result = match fetcher(1, per_page).await {
            Ok(result) => result,
            Err(e) => {
                let _ = self.cache.execute(|conn| {
                    MetadataSyncManager::complete_sync_with_error(
                        conn,
                        info.list_metadata_id,
                        &e.to_string(),
                    )
                });
                return Err(e);
            }
        };

        // 3. Store metadata (replacing existing)
        let items: Vec<ListMetadataItemInput> = result
            .metadata
            .iter()
            .map(|m| ListMetadataItemInput {
                entity_id: m.id,
                modified_gmt: m.modified_gmt.as_ref().map(|dt| dt.to_string()),
                parent: m.parent,
                menu_order: m.menu_order,
            })
            .collect();

        if let Err(e) = self.cache.execute(|conn| {
            ListMetadataRepository::set_items_by_list_metadata_id(conn, info.list_metadata_id, &items)
        }) {
            let _ = self.cache.execute(|conn| {
                MetadataSyncManager::complete_sync_with_error(
                    conn,
                    info.list_metadata_id,
                    &e.to_string(),
                )
            });
            return Err(e.into());
        }

        // 4. Update pagination
        if let Err(e) = self.cache.execute(|conn| {
            ListMetadataRepository::update_header_by_list_metadata_id(
                conn,
                info.list_metadata_id,
                &ListMetadataHeaderUpdate {
                    total_pages: result.total_pages.map(|p| p as i64),
                    total_items: result.total_items,
                    current_page: 1,
                    per_page: per_page as i64,
                },
            )
        }) {
            let _ = self.cache.execute(|conn| {
                MetadataSyncManager::complete_sync_with_error(
                    conn,
                    info.list_metadata_id,
                    &e.to_string(),
                )
            });
            return Err(e.into());
        }

        // 5. Set state to Idle
        self.cache.execute(|conn| {
            MetadataSyncManager::complete_sync(conn, info.list_metadata_id)
        })?;

        log::debug!("MetadataService::refresh: completed successfully, {} items", result.metadata.len());
        Ok(result)
    }

    // ============================================================
    // Sync Session API
    // ============================================================

    /// Begin a sync operation with RAII-based error cleanup.
    ///
    /// This creates a `SyncSession` that automatically marks the sync as failed
    /// if dropped without calling `complete()`. Use this for sync operations
    /// where you want automatic cleanup on early returns or errors.
    ///
    /// # Arguments
    /// * `service` - Arc reference to this service (for cleanup in Drop)
    /// * `key` - The list key to sync
    /// * `per_page` - Items per page setting
    /// * `is_first_page` - Whether this is a refresh (true) or load-more (false)
    ///
    /// # Example
    /// ```ignore
    /// let session = MetadataService::begin_sync(
    ///     self.metadata_service.clone(), &key, per_page, true
    /// )?;
    /// // Work with session...
    /// session.complete()?;
    /// ```
    pub fn begin_sync(
        service: Arc<Self>,
        key: &ListKey,
        per_page: i64,
        is_first_page: bool,
    ) -> Result<SyncSession, WpServiceError> {
        let (list_metadata_id, version) = if is_first_page {
            let info = service.cache.execute(|conn| {
                MetadataSyncManager::begin_refresh(conn, &service.db_site, key, per_page)
            })?;
            (info.list_metadata_id, info.version)
        } else {
            let info = service
                .cache
                .execute(|conn| {
                    MetadataSyncManager::begin_fetch_next_page(conn, &service.db_site, key)
                })?
                .ok_or_else(|| WpServiceError::DatabaseError {
                    err_message: "Cannot load more: no pages loaded or at last page".to_string(),
                })?;
            (info.list_metadata_id, info.version)
        };

        Ok(SyncSession::new(
            list_metadata_id,
            version,
            per_page,
            is_first_page,
            service,
        ))
    }

    /// Store items for a sync session.
    ///
    /// Replaces items if first page (refresh), appends if subsequent page.
    /// This is the preferred method when using `SyncSession`.
    pub fn store_for_session(
        &self,
        session: &SyncSession,
        key: &ListKey,
        metadata: &[EntityMetadata],
    ) -> Result<(), WpServiceError> {
        if session.is_first_page() {
            self.set_items(key, session.per_page(), metadata)
        } else {
            self.append_items(key, session.per_page(), metadata)
        }
    }

    /// Update pagination for a sync session.
    ///
    /// This is the preferred method when using `SyncSession`.
    pub fn update_pagination_for_session(
        &self,
        session: &SyncSession,
        key: &ListKey,
        total_pages: Option<i64>,
        total_items: Option<i64>,
        current_page: i64,
    ) -> Result<(), WpServiceError> {
        self.update_pagination(
            key,
            total_pages,
            total_items,
            current_page,
            session.per_page(),
        )
    }

    // ============================================================
    // Sync Completion (used by SyncSession and by-key methods)
    // ============================================================

    /// Complete a sync operation successfully (by list_metadata_id).
    ///
    /// Sets state to Idle. This is typically called by `SyncSession::complete()`.
    pub fn complete_sync(&self, list_metadata_id: RowId) -> Result<(), WpServiceError> {
        self.cache
            .execute(|conn| MetadataSyncManager::complete_sync(conn, list_metadata_id))?;
        Ok(())
    }

    /// Complete a sync operation successfully (by key).
    ///
    /// Sets state to Idle. Use this when you don't have the `list_metadata_id`.
    /// Does nothing if the list doesn't exist.
    pub fn complete_sync_by_key(&self, key: &ListKey) -> Result<(), WpServiceError> {
        use wp_mobile_cache::SqliteDbError;
        self.cache.execute(|conn| {
            if let Some(header) = ListMetadataRepository::get_header(conn, &self.db_site, key)? {
                MetadataSyncManager::complete_sync(conn, header.row_id)?;
            }
            Ok::<(), SqliteDbError>(())
        })?;
        Ok(())
    }

    /// Complete a sync operation with error (by list_metadata_id).
    ///
    /// Sets state to Error with the provided message. This is typically called
    /// by `SyncSession::drop()` for automatic error cleanup.
    pub fn complete_sync_with_error(
        &self,
        list_metadata_id: RowId,
        error_message: &str,
    ) -> Result<(), WpServiceError> {
        self.cache.execute(|conn| {
            MetadataSyncManager::complete_sync_with_error(conn, list_metadata_id, error_message)
        })?;
        Ok(())
    }

    /// Complete a sync operation with error (by key).
    ///
    /// Sets state to Error with the provided message. Use this when you don't have
    /// the `list_metadata_id`. Does nothing if the list doesn't exist.
    pub fn complete_sync_with_error_by_key(
        &self,
        key: &ListKey,
        error_message: &str,
    ) -> Result<(), WpServiceError> {
        use wp_mobile_cache::SqliteDbError;
        self.cache.execute(|conn| {
            if let Some(header) = ListMetadataRepository::get_header(conn, &self.db_site, key)? {
                MetadataSyncManager::complete_sync_with_error(conn, header.row_id, error_message)?;
            }
            Ok::<(), SqliteDbError>(())
        })?;
        Ok(())
    }
}

/// Implement ListMetadataReader for database-backed metadata.
///
/// This allows MetadataCollection to read list structure from the database
/// through the same trait interface it uses for in-memory stores.
impl ListMetadataReader for MetadataService {
    fn get_list_info(&self, key: &ListKey) -> Option<ListInfo> {
        self.cache
            .execute(|conn| ListMetadataRepository::get_header_with_state(conn, &self.db_site, key))
            .ok()
            .flatten()
            .map(|db| ListInfo {
                state: db.state,
                error_message: db.error_message,
                current_page: db.current_page,
                total_pages: db.total_pages,
                total_items: db.total_items,
                per_page: db.per_page,
            })
    }

    fn get_items(&self, key: &ListKey) -> Option<Vec<EntityMetadata>> {
        self.get_metadata(key).ok().flatten()
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

    const PER_PAGE: i64 = 20;

    #[rstest]
    fn test_get_entity_ids_returns_empty_for_non_existent(test_ctx: TestContext) {
        let key = ListKey::from("nonexistent");
        let ids = test_ctx.service.get_entity_ids(&key).unwrap();
        assert!(ids.is_empty());
    }

    #[rstest]
    fn test_get_metadata_returns_none_for_non_existent(test_ctx: TestContext) {
        let key = ListKey::from("nonexistent");
        let metadata = test_ctx.service.get_metadata(&key).unwrap();
        assert!(metadata.is_none());
    }

    #[rstest]
    fn test_set_and_get_items(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:publish");
        let metadata = vec![
            EntityMetadata::new(100, None, None, None),
            EntityMetadata::new(200, None, None, None),
            EntityMetadata::new(300, None, None, None),
        ];

        test_ctx
            .service
            .set_items(&key, PER_PAGE, &metadata)
            .unwrap();

        let ids = test_ctx.service.get_entity_ids(&key).unwrap();
        assert_eq!(ids, vec![100, 200, 300]);
    }

    #[rstest]
    fn test_set_items_replaces_existing(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:draft");

        test_ctx
            .service
            .set_items(
                &key,
                PER_PAGE,
                &[
                    EntityMetadata::new(1, None, None, None),
                    EntityMetadata::new(2, None, None, None),
                ],
            )
            .unwrap();

        test_ctx
            .service
            .set_items(
                &key,
                PER_PAGE,
                &[
                    EntityMetadata::new(10, None, None, None),
                    EntityMetadata::new(20, None, None, None),
                ],
            )
            .unwrap();

        let ids = test_ctx.service.get_entity_ids(&key).unwrap();
        assert_eq!(ids, vec![10, 20]);
    }

    #[rstest]
    fn test_append_items(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:pending");

        test_ctx
            .service
            .set_items(&key, PER_PAGE, &[EntityMetadata::new(1, None, None, None)])
            .unwrap();

        test_ctx
            .service
            .append_items(
                &key,
                PER_PAGE,
                &[
                    EntityMetadata::new(2, None, None, None),
                    EntityMetadata::new(3, None, None, None),
                ],
            )
            .unwrap();

        let ids = test_ctx.service.get_entity_ids(&key).unwrap();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[rstest]
    fn test_get_state_returns_idle_for_non_existent(test_ctx: TestContext) {
        let key = ListKey::from("nonexistent");
        let state = test_ctx.service.get_state(&key).unwrap();
        assert_eq!(state, ListState::Idle);
    }

    #[rstest]
    fn test_set_and_get_state(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:publish");

        test_ctx
            .service
            .set_state(&key, PER_PAGE, ListState::FetchingFirstPage, None)
            .unwrap();

        let state = test_ctx.service.get_state(&key).unwrap();
        assert_eq!(state, ListState::FetchingFirstPage);
    }

    #[rstest]
    fn test_update_and_get_pagination(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:publish");

        test_ctx
            .service
            .update_pagination(&key, Some(5), Some(100), 1, 20)
            .unwrap();

        let pagination = test_ctx.service.get_pagination(&key).unwrap().unwrap();
        assert_eq!(pagination.total_pages, Some(5));
        assert_eq!(pagination.total_items, Some(100));
        assert_eq!(pagination.current_page, 1);
        assert_eq!(pagination.per_page, 20);
    }

    #[rstest]
    fn test_has_more_pages(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:publish");

        // No pages loaded yet
        test_ctx
            .service
            .update_pagination(&key, Some(3), None, 0, 20)
            .unwrap();
        assert!(test_ctx.service.has_more_pages(&key).unwrap());

        // Page 1 of 3 loaded
        test_ctx
            .service
            .update_pagination(&key, Some(3), None, 1, 20)
            .unwrap();
        assert!(test_ctx.service.has_more_pages(&key).unwrap());

        // Page 3 of 3 loaded (no more)
        test_ctx
            .service
            .update_pagination(&key, Some(3), None, 3, 20)
            .unwrap();
        assert!(!test_ctx.service.has_more_pages(&key).unwrap());
    }

    // Note: begin_refresh and begin_fetch_next_page functionality is tested in
    // sync::metadata_sync_manager::tests. The MetadataService wraps these via
    // begin_sync() which returns a SyncSession.

    #[rstest]
    fn test_delete_list(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:publish");

        test_ctx
            .service
            .set_items(&key, PER_PAGE, &[EntityMetadata::new(1, None, None, None)])
            .unwrap();
        test_ctx
            .service
            .update_pagination(&key, Some(1), None, 1, 20)
            .unwrap();

        test_ctx.service.delete_list(&key).unwrap();

        assert!(test_ctx.service.get_metadata(&key).unwrap().is_none());
        assert!(test_ctx.service.get_pagination(&key).unwrap().is_none());
    }

    #[rstest]
    fn test_list_metadata_reader_get_items(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:publish");
        let metadata = vec![
            EntityMetadata::new(100, None, None, None),
            EntityMetadata::new(200, None, None, None),
        ];

        test_ctx
            .service
            .set_items(&key, PER_PAGE, &metadata)
            .unwrap();

        // Access via trait
        let reader: &dyn ListMetadataReader = &test_ctx.service;
        let result = reader.get_items(&key).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, 100);
        assert_eq!(result[1].id, 200);
    }

    #[rstest]
    fn test_list_metadata_reader_get_items_returns_none_for_non_existent(test_ctx: TestContext) {
        let key = ListKey::from("nonexistent");
        let reader: &dyn ListMetadataReader = &test_ctx.service;
        assert!(reader.get_items(&key).is_none());
    }

    #[rstest]
    fn test_list_metadata_reader_get_list_info(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:publish");

        // Initially no info
        let reader: &dyn ListMetadataReader = &test_ctx.service;
        assert!(reader.get_list_info(&key).is_none());

        // Create header via update_pagination (this creates the list metadata entry)
        test_ctx
            .service
            .update_pagination(&key, Some(5), Some(100), 1, 20)
            .unwrap();

        let info = reader.get_list_info(&key).unwrap();
        assert_eq!(info.current_page, 1);
        assert_eq!(info.per_page, 20);
        assert_eq!(info.total_pages, Some(5));
        assert_eq!(info.total_items, Some(100));
        assert_eq!(info.state, wp_mobile_cache::list_metadata::ListState::Idle);
    }

    #[rstest]
    fn test_list_metadata_reader_get_list_info_with_state(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:publish");
        let metadata = vec![EntityMetadata::new(100, None, None, None)];
        test_ctx
            .service
            .set_items(&key, PER_PAGE, &metadata)
            .unwrap();

        // Set state to FetchingFirstPage
        test_ctx
            .service
            .set_state(
                &key,
                PER_PAGE,
                wp_mobile_cache::list_metadata::ListState::FetchingFirstPage,
                None,
            )
            .unwrap();

        let reader: &dyn ListMetadataReader = &test_ctx.service;
        let info = reader.get_list_info(&key).unwrap();

        assert_eq!(
            info.state,
            wp_mobile_cache::list_metadata::ListState::FetchingFirstPage
        );
    }

    // ============================================================
    // Orchestration API tests (refresh)
    // ============================================================

    mod refresh_tests {
        use super::*;
        use crate::{collection::FetchError, sync::MetadataFetchResult};

        /// Helper to create a successful fetch result
        fn create_fetch_result(ids: Vec<i64>, total_pages: Option<u32>) -> MetadataFetchResult {
            let metadata = ids
                .into_iter()
                .map(|id| EntityMetadata::new(id, None, None, None))
                .collect();
            MetadataFetchResult::new(metadata, Some(100), total_pages, 1)
        }

        #[rstest]
        #[tokio::test]
        async fn test_refresh_stores_metadata_and_sets_state(test_ctx: TestContext) {
            let key = ListKey::from("test:refresh:basic");
            let fetch_result = create_fetch_result(vec![1, 2, 3], Some(5));

            let result = test_ctx
                .service
                .refresh(&key, 25, |_page, _per_page| async { Ok(fetch_result.clone()) })
                .await;

            assert!(result.is_ok());

            // Verify metadata was stored
            let ids = test_ctx.service.get_entity_ids(&key).unwrap();
            assert_eq!(ids, vec![1, 2, 3]);

            // Verify pagination was updated
            let pagination = test_ctx.service.get_pagination(&key).unwrap().unwrap();
            assert_eq!(pagination.current_page, 1);
            assert_eq!(pagination.total_pages, Some(5));
            assert_eq!(pagination.per_page, 25);

            // Verify state is Idle
            let state = test_ctx.service.get_state(&key).unwrap();
            assert_eq!(state, ListState::Idle);
        }

        #[rstest]
        #[tokio::test]
        async fn test_refresh_replaces_existing_metadata(test_ctx: TestContext) {
            let key = ListKey::from("test:refresh:replace");

            // First refresh
            let result1 = create_fetch_result(vec![1, 2, 3], Some(3));
            test_ctx
                .service
                .refresh(&key, 25, |_page, _per_page| async { Ok(result1.clone()) })
                .await
                .unwrap();

            // Second refresh should replace
            let result2 = create_fetch_result(vec![10, 20], Some(2));
            test_ctx
                .service
                .refresh(&key, 25, |_page, _per_page| async { Ok(result2.clone()) })
                .await
                .unwrap();

            let ids = test_ctx.service.get_entity_ids(&key).unwrap();
            assert_eq!(ids, vec![10, 20]);
        }

        #[rstest]
        #[tokio::test]
        async fn test_refresh_increments_version(test_ctx: TestContext) {
            let key = ListKey::from("test:refresh:version");
            let fetch_result = create_fetch_result(vec![1], Some(1));

            // First refresh
            test_ctx
                .service
                .refresh(&key, 25, |_page, _per_page| async { Ok(fetch_result.clone()) })
                .await
                .unwrap();

            let version1 = test_ctx.service.get_version(&key).unwrap();

            // Second refresh
            test_ctx
                .service
                .refresh(&key, 25, |_page, _per_page| async { Ok(fetch_result.clone()) })
                .await
                .unwrap();

            let version2 = test_ctx.service.get_version(&key).unwrap();
            assert!(version2 > version1, "Version should increment on refresh");
        }

        #[rstest]
        #[tokio::test]
        async fn test_refresh_sets_error_state_on_fetch_failure(test_ctx: TestContext) {
            let key = ListKey::from("test:refresh:error");

            let result = test_ctx
                .service
                .refresh(&key, 25, |_page, _per_page| async {
                    Err::<MetadataFetchResult, _>(FetchError::Database {
                        err_message: "Network error".to_string(),
                    })
                })
                .await;

            assert!(result.is_err());

            // Verify state is Error
            let state = test_ctx.service.get_state(&key).unwrap();
            assert_eq!(state, ListState::Error);
        }

        #[rstest]
        #[tokio::test]
        async fn test_refresh_passes_correct_page_and_per_page(test_ctx: TestContext) {
            use std::sync::atomic::{AtomicU32, Ordering};

            let key = ListKey::from("test:refresh:params");
            let received_page = Arc::new(AtomicU32::new(0));
            let received_per_page = Arc::new(AtomicU32::new(0));

            let page_clone = received_page.clone();
            let per_page_clone = received_per_page.clone();

            test_ctx
                .service
                .refresh(&key, 42, move |page, per_page| {
                    page_clone.store(page, Ordering::SeqCst);
                    per_page_clone.store(per_page, Ordering::SeqCst);
                    async { Ok(create_fetch_result(vec![1], Some(1))) }
                })
                .await
                .unwrap();

            assert_eq!(received_page.load(Ordering::SeqCst), 1);
            assert_eq!(received_per_page.load(Ordering::SeqCst), 42);
        }

        #[rstest]
        #[tokio::test]
        async fn test_refresh_can_recover_from_error(test_ctx: TestContext) {
            let key = ListKey::from("test:refresh:recover");

            // First refresh fails
            let _ = test_ctx
                .service
                .refresh(&key, 25, |_page, _per_page| async {
                    Err::<MetadataFetchResult, _>(FetchError::Database {
                        err_message: "Error".to_string(),
                    })
                })
                .await;

            assert_eq!(test_ctx.service.get_state(&key).unwrap(), ListState::Error);

            // Second refresh succeeds
            let fetch_result = create_fetch_result(vec![1, 2], Some(1));
            test_ctx
                .service
                .refresh(&key, 25, |_page, _per_page| async { Ok(fetch_result.clone()) })
                .await
                .unwrap();

            // State should be Idle now
            assert_eq!(test_ctx.service.get_state(&key).unwrap(), ListState::Idle);
            assert_eq!(test_ctx.service.get_entity_ids(&key).unwrap(), vec![1, 2]);
        }
    }
}
