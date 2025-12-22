//! RAII-based sync session for automatic error cleanup.
//!
//! This module provides `SyncSession`, which represents an active sync operation
//! with automatic error cleanup via the RAII pattern. When a session is dropped
//! without calling `complete()`, it automatically marks the sync as failed.
//!
//! # Design Rationale
//!
//! Using RAII for sync cleanup provides:
//! 1. **Automatic cleanup**: Early returns via `?` trigger cleanup automatically
//! 2. **Panic safety**: Even if a panic occurs, `Drop` runs and records the error
//! 3. **Explicit control flow**: No hidden control flow - code reads linearly
//! 4. **Easy to use**: "Create a session, do your work, complete the session"
//!
//! # Example
//!
//! ```ignore
//! let session = metadata_service.begin_sync(key, per_page, is_first_page)?;
//!
//! // Any error from here will auto-complete with error via Drop
//! let result = self.fetch_posts_metadata(...).await?;
//! self.metadata_service.store(&session, &result)?;
//!
//! // Explicit success - prevents Drop cleanup
//! session.complete()?;
//! ```

use std::sync::Arc;
use wp_mobile_cache::RowId;

use crate::service::{WpServiceError, metadata::MetadataService};

/// Represents an active sync operation with automatic error cleanup.
///
/// When a `SyncSession` is dropped without calling `complete()`, it
/// automatically marks the sync as failed. This ensures cleanup happens
/// even on panics or early `?` returns.
///
/// # Usage
///
/// ```ignore
/// let session = metadata_service.begin_sync(key, per_page, is_first_page)?;
///
/// // Any error from here will auto-complete with error via Drop
/// let result = self.fetch_posts_metadata(...).await?;
/// self.metadata_service.store(&session, &result)?;
/// self.metadata_service.update_pagination(&session, ...)?;
///
/// // Entity-specific processing
/// self.detect_and_mark_stale_posts(&result.metadata);
///
/// // Explicit success - prevents Drop cleanup
/// session.complete()?;
/// ```
pub struct SyncSession {
    /// The list metadata row ID for this sync operation
    list_metadata_id: RowId,

    /// Version at start of sync (for stale detection in load-more)
    version: i64,

    /// Items per page setting
    per_page: i64,

    /// Whether this is a first page (refresh) or subsequent page
    is_first_page: bool,

    /// Reference to service for cleanup
    metadata_service: Arc<MetadataService>,

    /// Whether complete() was called
    completed: bool,
}

impl SyncSession {
    /// Create a new sync session.
    ///
    /// This is typically called by `MetadataService::begin_sync()`.
    pub(crate) fn new(
        list_metadata_id: RowId,
        version: i64,
        per_page: i64,
        is_first_page: bool,
        metadata_service: Arc<MetadataService>,
    ) -> Self {
        Self {
            list_metadata_id,
            version,
            per_page,
            is_first_page,
            metadata_service,
            completed: false,
        }
    }

    /// Get the list metadata ID for operations that need it.
    pub fn list_metadata_id(&self) -> RowId {
        self.list_metadata_id
    }

    /// Get the version for stale detection.
    ///
    /// When completing a load-more operation, compare this version with
    /// the current version. If they differ, a refresh occurred and the
    /// results should be discarded.
    pub fn version(&self) -> i64 {
        self.version
    }

    /// Get the items per page setting.
    pub fn per_page(&self) -> i64 {
        self.per_page
    }

    /// Whether this is a first page (refresh) operation.
    pub fn is_first_page(&self) -> bool {
        self.is_first_page
    }

    /// Mark sync as successfully completed.
    ///
    /// This sets state to Idle and prevents the Drop cleanup.
    /// Call this after all sync operations have succeeded.
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails.
    pub fn complete(mut self) -> Result<(), WpServiceError> {
        self.completed = true;
        self.metadata_service.complete_sync(self.list_metadata_id)
    }
}

impl Drop for SyncSession {
    fn drop(&mut self) {
        if !self.completed {
            log::warn!(
                "SyncSession dropped without completion for list_metadata_id={}, marking as error",
                self.list_metadata_id.0
            );
            // Ignore errors during Drop - we can't propagate them anyway
            let _ = self.metadata_service.complete_sync_with_error(
                self.list_metadata_id,
                "Sync session dropped without completion (likely due to error)",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use rusqlite::Connection;
    use wp_mobile_cache::{
        MigrationManager, WpApiCache,
        db_types::{db_site::DbSite, self_hosted_site::SelfHostedSite},
        list_metadata::{ListKey, ListState},
        repository::sites::SiteRepository,
    };

    use crate::sync::MetadataSyncManager;

    struct TestContext {
        service: Arc<MetadataService>,
        cache: Arc<WpApiCache>,
        site: Arc<DbSite>,
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
        let site = Arc::new(db_site);
        let service = Arc::new(MetadataService::new(site.clone(), cache.clone()));

        TestContext {
            service,
            cache,
            site,
        }
    }

    const PER_PAGE: i64 = 25;

    // ============================================================
    // Basic functionality tests
    // ============================================================

    #[rstest]
    fn test_session_accessors(test_ctx: TestContext) {
        let key = ListKey::from("test:posts:publish");

        // Begin sync to get list_metadata_id
        let info = test_ctx
            .cache
            .execute(|conn| {
                MetadataSyncManager::begin_refresh(conn, &test_ctx.site, &key, PER_PAGE)
            })
            .expect("should succeed");

        let session = SyncSession::new(
            info.list_metadata_id,
            info.version,
            info.per_page,
            true,
            test_ctx.service.clone(),
        );

        assert_eq!(session.list_metadata_id(), info.list_metadata_id);
        assert_eq!(session.version(), info.version);
        assert_eq!(session.per_page(), PER_PAGE);
        assert!(session.is_first_page());

        // Cleanup
        session.complete().expect("should succeed");
    }

    #[rstest]
    fn test_complete_sets_state_to_idle(test_ctx: TestContext) {
        let key = ListKey::from("test:posts:complete");

        // Begin sync
        let info = test_ctx
            .cache
            .execute(|conn| {
                MetadataSyncManager::begin_refresh(conn, &test_ctx.site, &key, PER_PAGE)
            })
            .expect("should succeed");

        let session = SyncSession::new(
            info.list_metadata_id,
            info.version,
            info.per_page,
            true,
            test_ctx.service.clone(),
        );

        // Complete the session
        session.complete().expect("should succeed");

        // Verify state is Idle
        let state = test_ctx.service.get_state(&key).expect("should succeed");
        assert_eq!(state, ListState::Idle);
    }

    // ============================================================
    // RAII cleanup tests
    // ============================================================

    #[rstest]
    fn test_drop_without_complete_sets_error_state(test_ctx: TestContext) {
        let key = ListKey::from("test:posts:drop");

        // Begin sync
        let info = test_ctx
            .cache
            .execute(|conn| {
                MetadataSyncManager::begin_refresh(conn, &test_ctx.site, &key, PER_PAGE)
            })
            .expect("should succeed");

        // Create session and let it drop without completing
        {
            let _session = SyncSession::new(
                info.list_metadata_id,
                info.version,
                info.per_page,
                true,
                test_ctx.service.clone(),
            );
            // Session drops here without complete()
        }

        // Verify state is Error
        let state = test_ctx.service.get_state(&key).expect("should succeed");
        assert_eq!(state, ListState::Error);
    }

    #[rstest]
    fn test_drop_after_complete_does_not_set_error(test_ctx: TestContext) {
        let key = ListKey::from("test:posts:complete_then_drop");

        // Begin sync
        let info = test_ctx
            .cache
            .execute(|conn| {
                MetadataSyncManager::begin_refresh(conn, &test_ctx.site, &key, PER_PAGE)
            })
            .expect("should succeed");

        // Create session, complete it, then drop
        {
            let session = SyncSession::new(
                info.list_metadata_id,
                info.version,
                info.per_page,
                true,
                test_ctx.service.clone(),
            );
            session.complete().expect("should succeed");
            // Session is consumed by complete(), no drop happens
        }

        // Verify state is still Idle (not Error)
        let state = test_ctx.service.get_state(&key).expect("should succeed");
        assert_eq!(state, ListState::Idle);
    }

    #[rstest]
    fn test_early_return_triggers_cleanup() {
        // This test demonstrates the pattern - actual early return via ?
        // would be tested in integration tests with async code

        fn simulate_sync_with_error(
            service: Arc<MetadataService>,
            site: &DbSite,
            cache: &WpApiCache,
            key: &ListKey,
        ) -> Result<(), &'static str> {
            let info = cache
                .execute(|conn| MetadataSyncManager::begin_refresh(conn, site, key, PER_PAGE))
                .map_err(|_| "begin failed")?;

            let _session = SyncSession::new(
                info.list_metadata_id,
                info.version,
                info.per_page,
                true,
                service,
            );

            // Simulate an error that causes early return
            // The session will be dropped here, triggering cleanup
            Err("simulated fetch error")?;

            // This would never be reached
            #[allow(unreachable_code)]
            {
                _session.complete().map_err(|_| "complete failed")?;
                Ok(())
            }
        }

        // Setup
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
        let site = Arc::new(db_site);
        let service = Arc::new(MetadataService::new(site.clone(), cache.clone()));

        let key = ListKey::from("test:posts:early_return");

        // Run the function that returns early with error
        let result = simulate_sync_with_error(service.clone(), &site, &cache, &key);
        assert!(result.is_err());

        // Verify the session cleanup happened
        let state = service.get_state(&key).expect("should succeed");
        assert_eq!(state, ListState::Error);
    }

    #[rstest]
    fn test_subsequent_sync_after_error_works(test_ctx: TestContext) {
        let key = ListKey::from("test:posts:retry");

        // First sync fails (drops without complete)
        {
            let info = test_ctx
                .cache
                .execute(|conn| {
                    MetadataSyncManager::begin_refresh(conn, &test_ctx.site, &key, PER_PAGE)
                })
                .expect("should succeed");

            let _session = SyncSession::new(
                info.list_metadata_id,
                info.version,
                info.per_page,
                true,
                test_ctx.service.clone(),
            );
            // Drops without complete
        }
        assert_eq!(test_ctx.service.get_state(&key).unwrap(), ListState::Error);

        // Second sync succeeds
        {
            let info = test_ctx
                .cache
                .execute(|conn| {
                    MetadataSyncManager::begin_refresh(conn, &test_ctx.site, &key, PER_PAGE)
                })
                .expect("should succeed");

            let session = SyncSession::new(
                info.list_metadata_id,
                info.version,
                info.per_page,
                true,
                test_ctx.service.clone(),
            );
            session.complete().expect("should succeed");
        }

        // State should be Idle now
        assert_eq!(test_ctx.service.get_state(&key).unwrap(), ListState::Idle);
    }

    // ============================================================
    // is_first_page behavior tests
    // ============================================================

    #[rstest]
    fn test_session_for_load_more(test_ctx: TestContext) {
        let key = ListKey::from("test:posts:loadmore");

        // First do a refresh to set up pagination
        let refresh_info = test_ctx
            .cache
            .execute(|conn| {
                MetadataSyncManager::begin_refresh(conn, &test_ctx.site, &key, PER_PAGE)
            })
            .expect("should succeed");

        // Update pagination to simulate page 1 loaded with more pages
        test_ctx
            .service
            .update_pagination(&key, Some(3), Some(75), 1, PER_PAGE)
            .expect("should succeed");

        test_ctx
            .cache
            .execute(|conn| MetadataSyncManager::complete_sync(conn, refresh_info.list_metadata_id))
            .expect("should succeed");

        // Now begin a load-more
        let load_more_info = test_ctx
            .cache
            .execute(|conn| MetadataSyncManager::begin_fetch_next_page(conn, &test_ctx.site, &key))
            .expect("should succeed")
            .expect("should have more pages");

        let session = SyncSession::new(
            load_more_info.list_metadata_id,
            load_more_info.version,
            load_more_info.per_page,
            false, // Not first page
            test_ctx.service.clone(),
        );

        assert!(!session.is_first_page());
        assert_eq!(session.version(), refresh_info.version);

        session.complete().expect("should succeed");
    }
}
