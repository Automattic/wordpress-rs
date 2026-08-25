use std::{future::Future, sync::Arc};
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
    sync::{EntityMetadata, ListInfo, ListMetadataReader},
};

use super::WpServiceError;

/// First page number (1-indexed per WordPress convention)
const FIRST_PAGE: i64 = 1;

// ============================================================
// Internal types for workflow orchestration
// ============================================================

/// Information returned when starting a refresh operation.
#[derive(Debug, Clone)]
struct RefreshInfo {
    /// Row ID of the list_metadata record
    list_metadata_id: RowId,
}

/// Information returned when starting a load-next-page operation.
#[derive(Debug, Clone)]
struct LoadMoreInfo {
    /// Row ID of the list_metadata record
    list_metadata_id: RowId,
    /// Page number to fetch (1-indexed)
    page: u32,
    /// Version at start (check before storing results)
    version: i64,
    /// Items per page setting
    per_page: u32,
}

/// Service layer for list metadata operations.
///
/// Provides database-backed persistence for list structure (ordered entity IDs)
/// and pagination state that survives app restarts.
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

    /// Check whether a list contains a specific entity ID.
    ///
    /// More efficient than `get_entity_ids` when only a membership check is needed.
    pub fn list_contains_entity(
        &self,
        key: &ListKey,
        entity_id: i64,
    ) -> Result<bool, WpServiceError> {
        self.cache.execute(|conn| {
            Ok(ListMetadataRepository::contains_entity(
                conn,
                &self.db_site,
                key,
                entity_id,
            )?)
        })
    }

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

    /// Replace all items in a list with the given items.
    ///
    /// Looks up the list header and replaces all items in a single transaction.
    /// Returns `Ok(())` on success, or an error if the list doesn't exist.
    pub fn replace_list_items(
        &self,
        key: &ListKey,
        items: &[ListMetadataItemInput],
    ) -> Result<(), WpServiceError> {
        Ok(self.cache.execute(|conn| {
            let header =
                ListMetadataRepository::get_header(conn, &self.db_site, key)?.ok_or_else(|| {
                    wp_mobile_cache::SqliteDbError::SqliteError(format!("List '{}' not found", key))
                })?;
            ListMetadataRepository::set_items_by_list_metadata_id(conn, header.row_id, items)
        })?)
    }

    /// Remove specific items from a list by entity ID.
    ///
    /// Uses a targeted DELETE instead of replacing the entire list,
    /// avoiding race conditions with concurrent refresh or load-more operations.
    pub fn remove_list_items(
        &self,
        key: &ListKey,
        entity_ids: &[i64],
    ) -> Result<(), WpServiceError> {
        Ok(self.cache.execute(|conn| {
            let header =
                ListMetadataRepository::get_header(conn, &self.db_site, key)?.ok_or_else(|| {
                    wp_mobile_cache::SqliteDbError::SqliteError(format!("List '{}' not found", key))
                })?;
            ListMetadataRepository::remove_items_by_entity_ids(conn, header.row_id, entity_ids)
        })?)
    }

    /// Remove `entity_id` from every list whose key starts with `key_prefix`
    /// for this site.
    ///
    /// Used by service-level deletes (e.g. `MediaService::delete_media_permanently`)
    /// to scrub the deleted entity from every cached list immediately, so observers
    /// see the removal without waiting for a full refresh.
    ///
    /// Returns the number of rows removed across all lists. Returns `Ok(0)` if the
    /// entity wasn't referenced in any matching list.
    pub fn remove_entity_from_lists_with_key_prefix(
        &self,
        key_prefix: &str,
        entity_id: i64,
    ) -> Result<usize, WpServiceError> {
        Ok(self.cache.execute(|conn| {
            ListMetadataRepository::remove_entity_from_lists_with_key_prefix(
                conn,
                &self.db_site,
                key_prefix,
                entity_id,
            )
        })?)
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
                    EntityMetadata::new(
                        item.entity_id,
                        item.modified_gmt,
                        item.parent,
                        item.menu_order,
                    )
                })
                .collect();

            Ok(Some(metadata))
        })
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
                total_pages: h.total_pages.map(|p| p as u32),
                total_items: h.total_items,
                current_page: h.current_page.map(|p| p as u32),
                per_page: h.per_page as u32,
            }))
        })
    }

    /// Delete a list if its stored `per_page` doesn't match the expected value.
    ///
    /// When the app updates the `per_page` configuration for a collection,
    /// the existing list in the DB becomes incompatible because page boundaries
    /// are derived from `per_page`. This deletes the stale list so the next
    /// refresh recreates it with the correct `per_page`.
    ///
    /// No-op if the list doesn't exist or already has the expected `per_page`.
    pub(crate) fn delete_list_if_per_page_changed(
        &self,
        key: &ListKey,
        per_page: u32,
    ) -> Result<(), WpServiceError> {
        self.cache.execute(|conn| {
            if let Some(header) = ListMetadataRepository::get_header(conn, &self.db_site, key)?
                && header.per_page != per_page as i64
            {
                log::info!(
                    "per_page changed for key={} (stored={}, expected={}), deleting old list",
                    key,
                    header.per_page,
                    per_page
                );
                ListMetadataRepository::delete_list(conn, &self.db_site, key)?;
            }
            Ok(())
        })
    }

    /// Get the current version for concurrency checking.
    fn get_version(&self, key: &ListKey) -> Result<i64, WpServiceError> {
        self.cache
            .execute(|conn| ListMetadataRepository::get_version(conn, &self.db_site, key))
            .map_err(Into::into)
    }

    // ============================================================
    // Private error handling helpers
    // ============================================================

    /// Execute an operation and handle errors by setting sync error state.
    ///
    /// On error, sets the list state to Error with the error message and returns the error.
    /// If setting the error state fails, logs a warning but still returns the original error.
    fn execute_with_error_handling<T, E>(
        &self,
        list_metadata_id: RowId,
        operation: Result<T, E>,
        context: &str,
    ) -> Result<T, FetchError>
    where
        E: std::fmt::Display + Into<FetchError>,
    {
        match operation {
            Ok(result) => Ok(result),
            Err(e) => {
                let error_message = e.to_string();
                if let Err(cleanup_err) =
                    self.complete_sync_with_error(list_metadata_id, &error_message)
                {
                    log::warn!(
                        "Failed to set error state after {}: {}",
                        context,
                        cleanup_err
                    );
                }
                Err(e.into())
            }
        }
    }

    // ============================================================
    // Private workflow helpers (sync state transitions)
    // ============================================================

    /// Begin a refresh operation: increment version and set state to FetchingFirstPage.
    fn begin_refresh(&self, key: &ListKey, per_page: u32) -> Result<RefreshInfo, FetchError> {
        let header_info = self.cache.execute(|conn| {
            ListMetadataRepository::get_or_create_and_increment_version(
                conn,
                &self.db_site,
                key,
                per_page as i64,
            )
        })?;

        self.cache.execute(|conn| {
            ListMetadataRepository::update_state_by_list_metadata_id(
                conn,
                header_info.list_metadata_id,
                ListState::FetchingFirstPage,
                None,
            )
        })?;

        Ok(RefreshInfo {
            list_metadata_id: header_info.list_metadata_id,
        })
    }

    /// Begin a load-more operation: validate pagination and set state to FetchingNextPage.
    ///
    /// Returns `None` if cannot load more (list doesn't exist, no pages loaded,
    /// or already at last page).
    fn begin_load_more(&self, key: &ListKey) -> Result<Option<LoadMoreInfo>, FetchError> {
        let header = match self
            .cache
            .execute(|conn| ListMetadataRepository::get_header(conn, &self.db_site, key))?
        {
            Some(h) => h,
            None => return Ok(None), // List doesn't exist
        };

        // Must have loaded at least one page
        let current_page = match header.current_page {
            Some(p) => p,
            None => return Ok(None), // No pages loaded yet, need refresh first
        };

        // Check if there are more pages
        if let Some(total_pages) = header.total_pages
            && current_page >= total_pages
        {
            return Ok(None); // Already at last page
        }

        let next_page = current_page + 1;

        // Set state to FetchingNextPage
        self.cache.execute(|conn| {
            ListMetadataRepository::update_state_by_list_metadata_id(
                conn,
                header.row_id,
                ListState::FetchingNextPage,
                None,
            )
        })?;

        Ok(Some(LoadMoreInfo {
            list_metadata_id: header.row_id,
            page: next_page as u32,
            version: header.version,
            per_page: header.per_page as u32,
        }))
    }

    /// Complete a sync operation successfully: set state to Idle.
    fn complete_sync(&self, list_metadata_id: RowId) -> Result<(), FetchError> {
        self.cache
            .execute(|conn| {
                ListMetadataRepository::update_state_by_list_metadata_id(
                    conn,
                    list_metadata_id,
                    ListState::Idle,
                    None,
                )
            })
            .map_err(Into::into)
    }

    /// Complete a sync operation with error: set state to Error.
    fn complete_sync_with_error(
        &self,
        list_metadata_id: RowId,
        error_message: &str,
    ) -> Result<(), FetchError> {
        self.cache
            .execute(|conn| {
                ListMetadataRepository::update_state_by_list_metadata_id(
                    conn,
                    list_metadata_id,
                    ListState::Error,
                    Some(error_message),
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
        log::debug!(
            "MetadataService::refresh: key={}, per_page={}",
            key,
            per_page
        );

        // 1. Begin refresh (increment version, set state to FetchingFirstPage)
        let info = self.begin_refresh(key, per_page)?;

        // 2. Call fetcher with first page (if this fails, set error state)
        let result = self.execute_with_error_handling(
            info.list_metadata_id,
            fetcher(FIRST_PAGE as u32, per_page).await,
            "fetch failure",
        )?;

        // 3. Store metadata (replacing existing)
        // Convert domain EntityMetadata to database ListMetadataItemInput
        let items: Vec<ListMetadataItemInput> = result
            .metadata
            .iter()
            .map(|m| ListMetadataItemInput {
                entity_id: m.id,
                modified_gmt: m.modified_gmt,
                parent: m.parent,
                menu_order: m.menu_order,
            })
            .collect();

        self.execute_with_error_handling(
            info.list_metadata_id,
            self.cache.execute(|conn| {
                ListMetadataRepository::set_items_by_list_metadata_id(
                    conn,
                    info.list_metadata_id,
                    &items,
                )
            }),
            "store failure",
        )?;

        // 4. Update pagination
        self.execute_with_error_handling(
            info.list_metadata_id,
            self.cache.execute(|conn| {
                ListMetadataRepository::update_header_by_list_metadata_id(
                    conn,
                    info.list_metadata_id,
                    &ListMetadataHeaderUpdate {
                        total_pages: result.total_pages.map(|p| p as i64),
                        total_items: result.total_items,
                        current_page: Some(FIRST_PAGE),
                        per_page: per_page as i64,
                    },
                )
            }),
            "pagination update failure",
        )?;

        // 5. Set state to Idle
        self.complete_sync(info.list_metadata_id)?;

        log::debug!(
            "MetadataService::refresh: completed successfully, {} items",
            result.metadata.len()
        );
        Ok(result)
    }

    /// Load more items by fetching the next page and appending to existing data.
    ///
    /// Orchestrates the load-more lifecycle:
    /// 1. Get current state, determine next page
    /// 2. Verify there are more pages to load
    /// 3. Set state to FetchingNextPage
    /// 4. Call the fetcher with (next_page, per_page)
    /// 5. Check version (if refresh happened, discard results)
    /// 6. Append metadata to existing items
    /// 7. Update pagination
    /// 8. Set state to Idle (or Error on failure)
    ///
    /// # Arguments
    /// * `key` - The list key identifying which list to load more for
    /// * `fetcher` - Async closure that fetches metadata, receives (page, per_page)
    ///
    /// # Returns
    /// - `Ok(MetadataFetchResult)` on success
    /// - `Err(FetchError)` on failure (state is set to Error)
    ///
    /// # Errors
    /// - Returns error if list doesn't exist (must call `refresh` first)
    /// - Returns error if no more pages to load
    /// - Returns error if refresh happened during load-more (stale results)
    ///
    /// # Example
    /// ```ignore
    /// let result = metadata_service.load_more(
    ///     &key,
    ///     |page, per_page| async move {
    ///         api_client.fetch_metadata(page, per_page).await
    ///     },
    /// ).await?;
    /// ```
    pub async fn load_more<F, Fut>(
        &self,
        key: &ListKey,
        fetcher: F,
    ) -> Result<crate::sync::MetadataFetchResult, FetchError>
    where
        F: FnOnce(u32, u32) -> Fut,
        Fut: Future<Output = Result<crate::sync::MetadataFetchResult, FetchError>>,
    {
        log::debug!("MetadataService::load_more: key={}", key);

        // 1. Get current state and determine next page
        let load_more_info = self
            .begin_load_more(key)?
            .ok_or_else(|| FetchError::Database {
                err_message: "Cannot load more: list not found, no pages loaded, or at last page"
                    .to_string(),
            })?;

        let next_page = load_more_info.page;
        let per_page = load_more_info.per_page;
        let version = load_more_info.version;
        let list_metadata_id = load_more_info.list_metadata_id;

        log::debug!(
            "MetadataService::load_more: next_page={}, per_page={}, version={}",
            next_page,
            per_page,
            version
        );

        // 2. Call fetcher with next page (if this fails, set error state)
        let result = self.execute_with_error_handling(
            list_metadata_id,
            fetcher(next_page, per_page).await,
            "fetch failure",
        )?;

        // 3. Check version (refresh might have happened while fetching)
        let current_version = self.get_version(key)?;

        if current_version != version {
            log::warn!(
                "MetadataService::load_more: version mismatch (expected {}, got {}), discarding results",
                version,
                current_version
            );
            // Version mismatch means refresh was called while we were fetching (race condition).
            // Don't modify state - whoever called refresh owns the state transition.
            // Our fetched data is stale, so just discard it and return an error.
            return Err(FetchError::StaleLoadMore);
        }

        // 4. Append metadata to existing items
        // Convert domain EntityMetadata to database ListMetadataItemInput
        let items: Vec<ListMetadataItemInput> = result
            .metadata
            .iter()
            .map(|m| ListMetadataItemInput {
                entity_id: m.id,
                modified_gmt: m.modified_gmt,
                parent: m.parent,
                menu_order: m.menu_order,
            })
            .collect();

        self.execute_with_error_handling(
            list_metadata_id,
            self.cache.execute(|conn| {
                ListMetadataRepository::append_items_by_list_metadata_id(
                    conn,
                    list_metadata_id,
                    &items,
                )
            }),
            "append failure",
        )?;

        // 5. Update pagination
        self.execute_with_error_handling(
            list_metadata_id,
            self.cache.execute(|conn| {
                ListMetadataRepository::update_header_by_list_metadata_id(
                    conn,
                    list_metadata_id,
                    &ListMetadataHeaderUpdate {
                        total_pages: result.total_pages.map(|p| p as i64),
                        total_items: result.total_items,
                        current_page: Some(next_page as i64),
                        per_page: per_page as i64,
                    },
                )
            }),
            "pagination update failure",
        )?;

        // 6. Set state to Idle
        self.complete_sync(list_metadata_id)?;

        log::debug!(
            "MetadataService::load_more: completed successfully, {} items on page {}",
            result.metadata.len(),
            next_page
        );
        Ok(result)
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
                current_page: db.current_page.map(|p| p as u32),
                total_pages: db.total_pages.map(|p| p as u32),
                total_items: db.total_items,
                per_page: db.per_page as u32,
            })
    }

    fn get_items(&self, key: &ListKey) -> Option<Vec<EntityMetadata>> {
        self.get_metadata(key).ok().flatten()
    }
}

/// Pagination info for a list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListPaginationInfo {
    pub total_pages: Option<u32>,
    pub total_items: Option<i64>,
    pub current_page: Option<u32>,
    pub per_page: u32,
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
        /// Held to keep the cache alive for the lifetime of the service.
        /// Not directly accessed in tests, but must be retained.
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

        let cache = Arc::new(WpApiCache::try_from(conn).expect("Cache creation should succeed"));
        let db_site = Arc::new(db_site);
        let service = MetadataService::new(db_site, cache.clone());

        TestContext { service, cache }
    }

    #[rstest]
    fn test_get_entity_ids_returns_empty_for_non_existent(test_ctx: TestContext) {
        let key = ListKey::from("nonexistent");
        let ids = test_ctx
            .service
            .get_entity_ids(&key)
            .expect("Should get entity IDs");
        assert!(ids.is_empty());
    }

    #[rstest]
    fn test_get_metadata_returns_none_for_non_existent(test_ctx: TestContext) {
        let key = ListKey::from("nonexistent");
        let metadata = test_ctx
            .service
            .get_metadata(&key)
            .expect("Should get metadata");
        assert!(metadata.is_none());
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
            MetadataFetchResult::new(metadata, Some(100), total_pages, FIRST_PAGE as u32)
        }

        #[rstest]
        #[tokio::test]
        async fn test_refresh_stores_metadata_and_sets_state(test_ctx: TestContext) {
            let key = ListKey::from("test:refresh:basic");
            let fetch_result = create_fetch_result(vec![1, 2, 3], Some(5));

            test_ctx
                .service
                .refresh(&key, 25, |_page, _per_page| async move { Ok(fetch_result) })
                .await
                .expect("Refresh should succeed");

            // Verify metadata was stored
            let ids = test_ctx
                .service
                .get_entity_ids(&key)
                .expect("Should get entity IDs");
            assert_eq!(ids, vec![1, 2, 3]);

            // Verify pagination was updated
            let pagination = test_ctx
                .service
                .get_pagination(&key)
                .expect("Should get pagination")
                .expect("Pagination should exist");
            assert_eq!(pagination.current_page, Some(FIRST_PAGE as u32));
            assert_eq!(pagination.total_pages, Some(5));
            assert_eq!(pagination.per_page, 25);

            // Verify state is Idle
            let reader: &dyn ListMetadataReader = &test_ctx.service;
            let state = reader
                .get_list_info(&key)
                .expect("Should get list info")
                .state;
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
                .refresh(&key, 25, |_page, _per_page| async move { Ok(result1) })
                .await
                .expect("First refresh should succeed");

            // Second refresh should replace
            let result2 = create_fetch_result(vec![10, 20], Some(2));
            test_ctx
                .service
                .refresh(&key, 25, |_page, _per_page| async move { Ok(result2) })
                .await
                .expect("Second refresh should succeed");

            let ids = test_ctx
                .service
                .get_entity_ids(&key)
                .expect("Should get entity IDs");
            assert_eq!(ids, vec![10, 20]);
        }

        #[rstest]
        #[tokio::test]
        async fn test_refresh_increments_version(test_ctx: TestContext) {
            let key = ListKey::from("test:refresh:version");
            let fetch_result = create_fetch_result(vec![1], Some(1));

            // First refresh
            let result1 = fetch_result.clone();
            test_ctx
                .service
                .refresh(&key, 25, |_page, _per_page| async move { Ok(result1) })
                .await
                .expect("First refresh should succeed");

            let version1 = test_ctx
                .service
                .get_version(&key)
                .expect("Should get version");

            // Second refresh
            test_ctx
                .service
                .refresh(&key, 25, |_page, _per_page| async move { Ok(fetch_result) })
                .await
                .expect("Second refresh should succeed");

            let version2 = test_ctx
                .service
                .get_version(&key)
                .expect("Should get version");
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
            let reader: &dyn ListMetadataReader = &test_ctx.service;
            let state = reader
                .get_list_info(&key)
                .expect("Should get list info")
                .state;
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
                .expect("Refresh should succeed");

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

            let reader: &dyn ListMetadataReader = &test_ctx.service;
            assert_eq!(
                reader
                    .get_list_info(&key)
                    .expect("Should get list info")
                    .state,
                ListState::Error
            );

            // Second refresh succeeds
            let fetch_result = create_fetch_result(vec![1, 2], Some(1));
            test_ctx
                .service
                .refresh(&key, 25, |_page, _per_page| async move { Ok(fetch_result) })
                .await
                .expect("Second refresh should succeed");

            // State should be Idle now
            let reader: &dyn ListMetadataReader = &test_ctx.service;
            assert_eq!(
                reader
                    .get_list_info(&key)
                    .expect("Should get list info")
                    .state,
                ListState::Idle
            );
            assert_eq!(
                test_ctx
                    .service
                    .get_entity_ids(&key)
                    .expect("Should get entity IDs"),
                vec![1, 2]
            );
        }
    }

    // ============================================================
    // Orchestration API tests (load_more)
    // ============================================================

    mod load_more_tests {
        use super::*;
        use crate::{collection::FetchError, sync::MetadataFetchResult};

        /// Helper to create a fetch result for a specific page
        fn create_fetch_result(
            ids: Vec<i64>,
            total_pages: Option<u32>,
            page: u32,
        ) -> MetadataFetchResult {
            let metadata = ids
                .into_iter()
                .map(|id| EntityMetadata::new(id, None, None, None))
                .collect();
            MetadataFetchResult::new(metadata, Some(100), total_pages, page)
        }

        #[rstest]
        #[tokio::test]
        async fn test_load_more_appends_metadata(test_ctx: TestContext) {
            let key = ListKey::from("test:loadmore:basic");

            // First, do a refresh to load page 1
            let page1_result = create_fetch_result(vec![1, 2, 3], Some(3), 1);
            test_ctx
                .service
                .refresh(&key, 25, |_page, _per_page| async move { Ok(page1_result) })
                .await
                .expect("Refresh should succeed");

            // Now load more (page 2)
            let page2_result = create_fetch_result(vec![4, 5, 6], Some(3), 2);
            test_ctx
                .service
                .load_more(&key, |_page, _per_page| async move { Ok(page2_result) })
                .await
                .expect("Load more should succeed");

            // Verify metadata was appended
            let ids = test_ctx
                .service
                .get_entity_ids(&key)
                .expect("Should get entity IDs");
            assert_eq!(ids, vec![1, 2, 3, 4, 5, 6]);

            // Verify pagination was updated
            let pagination = test_ctx
                .service
                .get_pagination(&key)
                .expect("Should get pagination")
                .expect("Pagination should exist");
            assert_eq!(pagination.current_page, Some(2));

            // Verify state is Idle
            let reader: &dyn ListMetadataReader = &test_ctx.service;
            let state = reader
                .get_list_info(&key)
                .expect("Should get list info")
                .state;
            assert_eq!(state, ListState::Idle);
        }

        #[rstest]
        #[tokio::test]
        async fn test_load_more_fails_without_prior_refresh(test_ctx: TestContext) {
            let key = ListKey::from("test:loadmore:norefresh");

            // Try to load more without refresh
            let result = test_ctx
                .service
                .load_more(&key, |_page, _per_page| async {
                    Ok(create_fetch_result(vec![1], Some(1), 2))
                })
                .await;

            assert!(result.is_err());
            assert!(
                result
                    .expect_err("Should fail without refresh")
                    .to_string()
                    .contains("Cannot load more")
            );
        }

        #[rstest]
        #[tokio::test]
        async fn test_load_more_fails_at_last_page(test_ctx: TestContext) {
            let key = ListKey::from("test:loadmore:lastpage");

            // Refresh with total_pages = 1 (only one page)
            let page1_result = create_fetch_result(vec![1, 2, 3], Some(1), 1);
            test_ctx
                .service
                .refresh(&key, 25, |_page, _per_page| async move { Ok(page1_result) })
                .await
                .expect("Refresh should succeed");

            // Try to load more
            let result = test_ctx
                .service
                .load_more(&key, |_page, _per_page| async {
                    Ok(create_fetch_result(vec![4], Some(1), 2))
                })
                .await;

            assert!(result.is_err());
            assert!(
                result
                    .expect_err("Should fail at last page")
                    .to_string()
                    .contains("Cannot load more")
            );
        }

        #[rstest]
        #[tokio::test]
        async fn test_load_more_passes_correct_page_number(test_ctx: TestContext) {
            use std::sync::atomic::{AtomicU32, Ordering};

            let key = ListKey::from("test:loadmore:page");

            // First refresh to page 1
            let page1_result = create_fetch_result(vec![1], Some(3), 1);
            test_ctx
                .service
                .refresh(&key, 25, |_page, _per_page| async move { Ok(page1_result) })
                .await
                .expect("Refresh should succeed");

            // Load more - should request page 2
            let received_page = Arc::new(AtomicU32::new(0));
            let page_clone = received_page.clone();

            test_ctx
                .service
                .load_more(&key, move |page, _per_page| {
                    page_clone.store(page, Ordering::SeqCst);
                    async { Ok(create_fetch_result(vec![2], Some(3), 2)) }
                })
                .await
                .expect("Load more should succeed");

            assert_eq!(received_page.load(Ordering::SeqCst), 2);

            // Load more again - should request page 3
            let received_page = Arc::new(AtomicU32::new(0));
            let page_clone = received_page.clone();

            test_ctx
                .service
                .load_more(&key, move |page, _per_page| {
                    page_clone.store(page, Ordering::SeqCst);
                    async { Ok(create_fetch_result(vec![3], Some(3), 3)) }
                })
                .await
                .expect("Load more should succeed");

            assert_eq!(received_page.load(Ordering::SeqCst), 3);
        }

        #[rstest]
        #[tokio::test]
        async fn test_load_more_sets_error_on_fetch_failure(test_ctx: TestContext) {
            let key = ListKey::from("test:loadmore:error");

            // First refresh
            let page1_result = create_fetch_result(vec![1], Some(3), 1);
            test_ctx
                .service
                .refresh(&key, 25, |_page, _per_page| async move { Ok(page1_result) })
                .await
                .expect("Refresh should succeed");

            // Load more fails
            let result = test_ctx
                .service
                .load_more(&key, |_page, _per_page| async {
                    Err::<MetadataFetchResult, _>(FetchError::Database {
                        err_message: "Network error".to_string(),
                    })
                })
                .await;

            assert!(result.is_err());

            // Verify state is Error
            let reader: &dyn ListMetadataReader = &test_ctx.service;
            let state = reader
                .get_list_info(&key)
                .expect("Should get list info")
                .state;
            assert_eq!(state, ListState::Error);
        }

        #[rstest]
        #[tokio::test]
        async fn test_load_more_uses_per_page_from_refresh(test_ctx: TestContext) {
            use std::sync::atomic::{AtomicU32, Ordering};

            let key = ListKey::from("test:loadmore:perpage");

            // Refresh with per_page = 50
            let page1_result = create_fetch_result(vec![1], Some(3), 1);
            test_ctx
                .service
                .refresh(&key, 50, |_page, _per_page| async move { Ok(page1_result) })
                .await
                .expect("Refresh should succeed");

            // Load more should use same per_page
            let received_per_page = Arc::new(AtomicU32::new(0));
            let per_page_clone = received_per_page.clone();

            test_ctx
                .service
                .load_more(&key, move |_page, per_page| {
                    per_page_clone.store(per_page, Ordering::SeqCst);
                    async { Ok(create_fetch_result(vec![2], Some(3), 2)) }
                })
                .await
                .expect("Load more should succeed");

            assert_eq!(received_per_page.load(Ordering::SeqCst), 50);
        }

        #[rstest]
        #[tokio::test]
        async fn test_load_more_does_not_increment_version(test_ctx: TestContext) {
            let key = ListKey::from("test:loadmore:version");

            // Refresh
            let page1_result = create_fetch_result(vec![1], Some(3), 1);
            test_ctx
                .service
                .refresh(&key, 25, |_page, _per_page| async move { Ok(page1_result) })
                .await
                .expect("Refresh should succeed");

            let version_after_refresh = test_ctx
                .service
                .get_version(&key)
                .expect("Should get version");

            // Load more
            let page2_result = create_fetch_result(vec![2], Some(3), 2);
            test_ctx
                .service
                .load_more(&key, |_page, _per_page| async move { Ok(page2_result) })
                .await
                .expect("Load more should succeed");

            let version_after_load_more = test_ctx
                .service
                .get_version(&key)
                .expect("Should get version");

            // Version should not change on load_more
            assert_eq!(version_after_refresh, version_after_load_more);
        }
    }
}
