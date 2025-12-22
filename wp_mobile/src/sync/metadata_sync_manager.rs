//! Sync workflow orchestration for list metadata.
//!
//! This module provides `MetadataSyncManager`, which manages sync workflow state
//! transitions for list metadata. It composes repository primitives into workflow
//! operations, separating orchestration logic from pure data access.
//!
//! # Architecture
//!
//! ```text
//! MetadataService (owns connection, manages sessions)
//!        │
//!        │ calls within cache.execute()
//!        ▼
//! MetadataSyncManager (stateless workflow operations)
//!        │
//!        │ composes
//!        ▼
//! ListMetadataRepository (SQL primitives)
//! ```
//!
//! # Design Rationale
//!
//! The repository layer (`ListMetadataRepository`) should contain only pure SQL
//! operations. Workflow logic like "increment version, then set state to fetching"
//! belongs in this manager layer.
//!
//! This separation:
//! 1. Keeps repositories testable as pure data access
//! 2. Makes workflow logic explicit and auditable
//! 3. Enables reuse across different entity services (posts, comments, etc.)

use wp_mobile_cache::{
    RowId, SqliteDbError,
    db_types::db_site::DbSite,
    list_metadata::{ListKey, ListState},
    repository::{QueryExecutor, list_metadata::ListMetadataRepository},
};

/// Manages sync workflow state transitions for list metadata.
///
/// This is a stateless helper that composes repository primitives into workflow
/// operations. All functions are associated (no `self`) and take a `QueryExecutor`
/// so they can be called within a transaction context.
///
/// # Responsibilities
///
/// - Starting refresh operations (increment version, set fetching state)
/// - Starting load-more operations (validate pagination, set fetching state)
/// - Completing sync operations (set idle/error state)
///
/// # Usage
///
/// Typically called from `MetadataService` within a `cache.execute()` block:
///
/// ```ignore
/// let info = cache.execute(|conn| {
///     MetadataSyncManager::begin_refresh(conn, &site, &key, per_page)
/// })?;
/// ```
pub struct MetadataSyncManager;

/// Information returned when starting a refresh operation.
#[derive(Debug, Clone)]
pub struct RefreshInfo {
    /// Row ID of the list_metadata record
    pub list_metadata_id: RowId,
    /// New version number (for concurrency checking)
    pub version: i64,
    /// Items per page setting
    pub per_page: i64,
}

/// Information returned when starting a load-next-page operation.
#[derive(Debug, Clone)]
pub struct FetchNextPageInfo {
    /// Row ID of the list_metadata record
    pub list_metadata_id: RowId,
    /// Page number to fetch
    pub page: i64,
    /// Version at start (check before storing results)
    pub version: i64,
    /// Items per page setting
    pub per_page: i64,
}

impl MetadataSyncManager {
    /// Begin a refresh operation (fetch first page).
    ///
    /// Atomically:
    /// 1. Creates the list header if needed
    /// 2. Increments version (invalidates any in-flight load-more)
    /// 3. Sets state to FetchingFirstPage
    ///
    /// Returns info needed for the API call and session creation.
    ///
    /// # Errors
    ///
    /// Returns `SqliteDbError::PerPageMismatch` if the existing header has a
    /// different `per_page` than requested.
    pub fn begin_refresh(
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &ListKey,
        per_page: i64,
    ) -> Result<RefreshInfo, SqliteDbError> {
        log::debug!("MetadataSyncManager::begin_refresh: key={}", key);

        // Atomic: get or create header with incremented version
        let header_info = ListMetadataRepository::get_or_create_and_increment_version(
            executor, site, key, per_page,
        )?;

        // Update state to fetching
        ListMetadataRepository::update_state_by_list_metadata_id(
            executor,
            header_info.list_metadata_id,
            ListState::FetchingFirstPage,
            None,
        )?;

        Ok(RefreshInfo {
            list_metadata_id: header_info.list_metadata_id,
            version: header_info.version,
            per_page: header_info.per_page,
        })
    }

    /// Begin a load-next-page operation.
    ///
    /// Validates:
    /// - List exists
    /// - At least one page has been loaded
    /// - There are more pages to load
    ///
    /// Returns `None` if cannot load more (list doesn't exist, no pages loaded,
    /// or already at last page). Returns `Some(info)` with the page to fetch
    /// and version for staleness checking.
    ///
    /// # Version Checking
    ///
    /// The returned `version` should be checked before storing results. If a
    /// refresh started while this load-more was in flight, the version will
    /// have changed and the results should be discarded.
    pub fn begin_fetch_next_page(
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &ListKey,
    ) -> Result<Option<FetchNextPageInfo>, SqliteDbError> {
        log::debug!("MetadataSyncManager::begin_fetch_next_page: key={}", key);

        let header = match ListMetadataRepository::get_header(executor, site, key)? {
            Some(h) => h,
            None => return Ok(None), // List doesn't exist
        };

        // Must have loaded at least one page
        if header.current_page == 0 {
            return Ok(None); // No pages loaded yet, need refresh first
        }

        // Check if there are more pages
        if let Some(total_pages) = header.total_pages
            && header.current_page >= total_pages
        {
            return Ok(None); // Already at last page
        }

        let next_page = header.current_page + 1;

        // Update state to fetching
        ListMetadataRepository::update_state_by_list_metadata_id(
            executor,
            header.row_id,
            ListState::FetchingNextPage,
            None,
        )?;

        Ok(Some(FetchNextPageInfo {
            list_metadata_id: header.row_id,
            page: next_page,
            version: header.version,
            per_page: header.per_page,
        }))
    }

    /// Complete a sync operation successfully.
    ///
    /// Sets state to Idle and clears any error message.
    pub fn complete_sync(
        executor: &impl QueryExecutor,
        list_metadata_id: RowId,
    ) -> Result<(), SqliteDbError> {
        log::debug!(
            "MetadataSyncManager::complete_sync: id={}",
            list_metadata_id.0
        );
        ListMetadataRepository::update_state_by_list_metadata_id(
            executor,
            list_metadata_id,
            ListState::Idle,
            None,
        )
    }

    /// Complete a sync operation with an error.
    ///
    /// Sets state to Error with the provided message.
    pub fn complete_sync_with_error(
        executor: &impl QueryExecutor,
        list_metadata_id: RowId,
        error_message: &str,
    ) -> Result<(), SqliteDbError> {
        log::debug!(
            "MetadataSyncManager::complete_sync_with_error: id={}, error={}",
            list_metadata_id.0,
            error_message
        );
        ListMetadataRepository::update_state_by_list_metadata_id(
            executor,
            list_metadata_id,
            ListState::Error,
            Some(error_message),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use wp_mobile_cache::test_fixtures::{TestContext, test_ctx};

    const TEST_PER_PAGE: i64 = 25;

    // ============================================================
    // begin_refresh tests
    // ============================================================

    #[rstest]
    fn test_begin_refresh_creates_header_and_sets_state(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:publish");

        let info =
            MetadataSyncManager::begin_refresh(&test_ctx.conn, &test_ctx.site, &key, TEST_PER_PAGE)
                .expect("should succeed");

        // Should have version 1 (first refresh)
        assert_eq!(info.version, 1);
        assert_eq!(info.per_page, TEST_PER_PAGE);

        // State should be FetchingFirstPage
        let state =
            ListMetadataRepository::get_state_by_list_key(&test_ctx.conn, &test_ctx.site, &key)
                .expect("should succeed");
        assert_eq!(state, ListState::FetchingFirstPage);
    }

    #[rstest]
    fn test_begin_refresh_increments_version_on_subsequent_calls(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:draft");

        // First refresh
        let info1 =
            MetadataSyncManager::begin_refresh(&test_ctx.conn, &test_ctx.site, &key, TEST_PER_PAGE)
                .expect("should succeed");
        assert_eq!(info1.version, 1);

        // Complete the sync
        MetadataSyncManager::complete_sync(&test_ctx.conn, info1.list_metadata_id)
            .expect("should succeed");

        // Second refresh should increment version
        let info2 =
            MetadataSyncManager::begin_refresh(&test_ctx.conn, &test_ctx.site, &key, TEST_PER_PAGE)
                .expect("should succeed");
        assert_eq!(info2.version, 2);
        assert_eq!(info2.list_metadata_id, info1.list_metadata_id);
    }

    #[rstest]
    fn test_begin_refresh_fails_on_per_page_mismatch(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:mismatch");

        // First refresh with per_page = 25
        MetadataSyncManager::begin_refresh(&test_ctx.conn, &test_ctx.site, &key, 25)
            .expect("should succeed");

        // Second refresh with different per_page should fail
        let result = MetadataSyncManager::begin_refresh(&test_ctx.conn, &test_ctx.site, &key, 10);
        assert!(matches!(
            result,
            Err(SqliteDbError::PerPageMismatch {
                expected: 10,
                actual: 25
            })
        ));
    }

    // ============================================================
    // begin_fetch_next_page tests
    // ============================================================

    #[rstest]
    fn test_begin_fetch_next_page_returns_none_for_nonexistent_list(test_ctx: TestContext) {
        let key = ListKey::from("nonexistent:list");

        let result =
            MetadataSyncManager::begin_fetch_next_page(&test_ctx.conn, &test_ctx.site, &key)
                .expect("should succeed");
        assert!(result.is_none());
    }

    #[rstest]
    fn test_begin_fetch_next_page_returns_none_when_no_pages_loaded(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:pending");

        // Create header but don't load any pages (current_page = 0)
        ListMetadataRepository::get_or_create(&test_ctx.conn, &test_ctx.site, &key, TEST_PER_PAGE)
            .expect("should succeed");

        let result =
            MetadataSyncManager::begin_fetch_next_page(&test_ctx.conn, &test_ctx.site, &key)
                .expect("should succeed");
        assert!(result.is_none());
    }

    #[rstest]
    fn test_begin_fetch_next_page_returns_none_at_last_page(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:lastpage");

        // Create header and set pagination showing we're at last page
        let list_id = ListMetadataRepository::get_or_create(
            &test_ctx.conn,
            &test_ctx.site,
            &key,
            TEST_PER_PAGE,
        )
        .expect("should succeed");

        use wp_mobile_cache::repository::list_metadata::ListMetadataHeaderUpdate;
        ListMetadataRepository::update_header_by_list_metadata_id(
            &test_ctx.conn,
            list_id,
            &ListMetadataHeaderUpdate {
                total_pages: Some(3),
                total_items: Some(75),
                current_page: 3, // Already at last page
                per_page: TEST_PER_PAGE,
            },
        )
        .expect("should succeed");

        let result =
            MetadataSyncManager::begin_fetch_next_page(&test_ctx.conn, &test_ctx.site, &key)
                .expect("should succeed");
        assert!(result.is_none());
    }

    #[rstest]
    fn test_begin_fetch_next_page_returns_info_when_more_pages_available(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:morepages");

        // Create header with current_page = 1, total_pages = 3
        let list_id = ListMetadataRepository::get_or_create(
            &test_ctx.conn,
            &test_ctx.site,
            &key,
            TEST_PER_PAGE,
        )
        .expect("should succeed");

        use wp_mobile_cache::repository::list_metadata::ListMetadataHeaderUpdate;
        ListMetadataRepository::update_header_by_list_metadata_id(
            &test_ctx.conn,
            list_id,
            &ListMetadataHeaderUpdate {
                total_pages: Some(3),
                total_items: Some(75),
                current_page: 1,
                per_page: TEST_PER_PAGE,
            },
        )
        .expect("should succeed");

        let result =
            MetadataSyncManager::begin_fetch_next_page(&test_ctx.conn, &test_ctx.site, &key)
                .expect("should succeed");

        let info = result.expect("should return Some");
        assert_eq!(info.list_metadata_id, list_id);
        assert_eq!(info.page, 2); // Next page
        assert_eq!(info.per_page, TEST_PER_PAGE);

        // State should be FetchingNextPage
        let state =
            ListMetadataRepository::get_state_by_list_key(&test_ctx.conn, &test_ctx.site, &key)
                .expect("should succeed");
        assert_eq!(state, ListState::FetchingNextPage);
    }

    #[rstest]
    fn test_begin_fetch_next_page_works_when_total_pages_unknown(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:unknowntotal");

        // Create header with current_page = 1 but no total_pages
        let list_id = ListMetadataRepository::get_or_create(
            &test_ctx.conn,
            &test_ctx.site,
            &key,
            TEST_PER_PAGE,
        )
        .expect("should succeed");

        use wp_mobile_cache::repository::list_metadata::ListMetadataHeaderUpdate;
        ListMetadataRepository::update_header_by_list_metadata_id(
            &test_ctx.conn,
            list_id,
            &ListMetadataHeaderUpdate {
                total_pages: None, // Unknown
                total_items: None,
                current_page: 1,
                per_page: TEST_PER_PAGE,
            },
        )
        .expect("should succeed");

        // Should still allow fetching next page when total is unknown
        let result =
            MetadataSyncManager::begin_fetch_next_page(&test_ctx.conn, &test_ctx.site, &key)
                .expect("should succeed");

        let info = result.expect("should return Some");
        assert_eq!(info.page, 2);
    }

    // ============================================================
    // complete_sync tests
    // ============================================================

    #[rstest]
    fn test_complete_sync_sets_state_to_idle(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:complete");

        // Start a refresh
        let info =
            MetadataSyncManager::begin_refresh(&test_ctx.conn, &test_ctx.site, &key, TEST_PER_PAGE)
                .expect("should succeed");

        // Verify state is FetchingFirstPage
        let state =
            ListMetadataRepository::get_state_by_list_key(&test_ctx.conn, &test_ctx.site, &key)
                .expect("should succeed");
        assert_eq!(state, ListState::FetchingFirstPage);

        // Complete the sync
        MetadataSyncManager::complete_sync(&test_ctx.conn, info.list_metadata_id)
            .expect("should succeed");

        // State should now be Idle
        let state =
            ListMetadataRepository::get_state_by_list_key(&test_ctx.conn, &test_ctx.site, &key)
                .expect("should succeed");
        assert_eq!(state, ListState::Idle);
    }

    // ============================================================
    // complete_sync_with_error tests
    // ============================================================

    #[rstest]
    fn test_complete_sync_with_error_sets_state_and_message(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:error");

        // Start a refresh
        let info =
            MetadataSyncManager::begin_refresh(&test_ctx.conn, &test_ctx.site, &key, TEST_PER_PAGE)
                .expect("should succeed");

        // Complete with error
        let error_msg = "Network timeout";
        MetadataSyncManager::complete_sync_with_error(
            &test_ctx.conn,
            info.list_metadata_id,
            error_msg,
        )
        .expect("should succeed");

        // Verify state and error message
        let state_record = ListMetadataRepository::get_state_by_list_metadata_id(
            &test_ctx.conn,
            info.list_metadata_id,
        )
        .expect("query should succeed")
        .expect("state should exist");

        assert_eq!(state_record.state, ListState::Error);
        assert_eq!(state_record.error_message.as_deref(), Some(error_msg));
    }

    #[rstest]
    fn test_complete_sync_clears_previous_error(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:clearerror");

        // Start refresh and complete with error
        let info1 =
            MetadataSyncManager::begin_refresh(&test_ctx.conn, &test_ctx.site, &key, TEST_PER_PAGE)
                .expect("should succeed");
        MetadataSyncManager::complete_sync_with_error(
            &test_ctx.conn,
            info1.list_metadata_id,
            "First error",
        )
        .expect("should succeed");

        // Start another refresh and complete successfully
        let info2 =
            MetadataSyncManager::begin_refresh(&test_ctx.conn, &test_ctx.site, &key, TEST_PER_PAGE)
                .expect("should succeed");
        MetadataSyncManager::complete_sync(&test_ctx.conn, info2.list_metadata_id)
            .expect("should succeed");

        // Error should be cleared
        let state_record = ListMetadataRepository::get_state_by_list_metadata_id(
            &test_ctx.conn,
            info2.list_metadata_id,
        )
        .expect("query should succeed")
        .expect("state should exist");

        assert_eq!(state_record.state, ListState::Idle);
        assert!(state_record.error_message.is_none());
    }

    // ============================================================
    // Workflow integration tests
    // ============================================================

    #[rstest]
    fn test_full_refresh_then_load_more_workflow(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:workflow");

        // 1. Begin refresh
        let refresh_info =
            MetadataSyncManager::begin_refresh(&test_ctx.conn, &test_ctx.site, &key, TEST_PER_PAGE)
                .expect("should succeed");
        assert_eq!(refresh_info.version, 1);

        // 2. Simulate storing results and updating pagination
        use wp_mobile_cache::repository::list_metadata::ListMetadataHeaderUpdate;
        ListMetadataRepository::update_header_by_list_metadata_id(
            &test_ctx.conn,
            refresh_info.list_metadata_id,
            &ListMetadataHeaderUpdate {
                total_pages: Some(5),
                total_items: Some(100),
                current_page: 1,
                per_page: TEST_PER_PAGE,
            },
        )
        .expect("should succeed");

        // 3. Complete refresh
        MetadataSyncManager::complete_sync(&test_ctx.conn, refresh_info.list_metadata_id)
            .expect("should succeed");

        // 4. Begin load more
        let load_more_info =
            MetadataSyncManager::begin_fetch_next_page(&test_ctx.conn, &test_ctx.site, &key)
                .expect("should succeed")
                .expect("should have more pages");

        assert_eq!(load_more_info.page, 2);
        assert_eq!(load_more_info.version, 1); // Version unchanged

        // 5. Update pagination after load more
        ListMetadataRepository::update_header_by_list_metadata_id(
            &test_ctx.conn,
            load_more_info.list_metadata_id,
            &ListMetadataHeaderUpdate {
                total_pages: Some(5),
                total_items: Some(100),
                current_page: 2,
                per_page: TEST_PER_PAGE,
            },
        )
        .expect("should succeed");

        // 6. Complete load more
        MetadataSyncManager::complete_sync(&test_ctx.conn, load_more_info.list_metadata_id)
            .expect("should succeed");

        // Final state should be Idle
        let state =
            ListMetadataRepository::get_state_by_list_key(&test_ctx.conn, &test_ctx.site, &key)
                .expect("should succeed");
        assert_eq!(state, ListState::Idle);
    }

    #[rstest]
    fn test_refresh_during_load_more_invalidates_version(test_ctx: TestContext) {
        let key = ListKey::from("edit:posts:concurrent");

        // 1. Set up initial state with page 1 loaded
        let refresh1 =
            MetadataSyncManager::begin_refresh(&test_ctx.conn, &test_ctx.site, &key, TEST_PER_PAGE)
                .expect("should succeed");

        use wp_mobile_cache::repository::list_metadata::ListMetadataHeaderUpdate;
        ListMetadataRepository::update_header_by_list_metadata_id(
            &test_ctx.conn,
            refresh1.list_metadata_id,
            &ListMetadataHeaderUpdate {
                total_pages: Some(5),
                total_items: Some(100),
                current_page: 1,
                per_page: TEST_PER_PAGE,
            },
        )
        .expect("should succeed");
        MetadataSyncManager::complete_sync(&test_ctx.conn, refresh1.list_metadata_id)
            .expect("should succeed");

        // 2. Start load-more (simulating in-flight request)
        let load_more =
            MetadataSyncManager::begin_fetch_next_page(&test_ctx.conn, &test_ctx.site, &key)
                .expect("should succeed")
                .expect("should have more pages");
        let original_version = load_more.version;

        // 3. User pulls to refresh while load-more is in flight
        let refresh2 =
            MetadataSyncManager::begin_refresh(&test_ctx.conn, &test_ctx.site, &key, TEST_PER_PAGE)
                .expect("should succeed");

        // Version should have incremented
        assert!(refresh2.version > original_version);

        // 4. When load-more completes, caller should check version
        // and discard results if it changed (this logic is in MetadataService,
        // not tested here - we just verify the version changed)
        let current_header =
            ListMetadataRepository::get_header(&test_ctx.conn, &test_ctx.site, &key)
                .expect("should succeed")
                .expect("should exist");
        assert_eq!(current_header.version, refresh2.version);
        assert!(current_header.version > original_version);
    }
}
