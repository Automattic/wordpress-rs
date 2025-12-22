# Sync Architecture Refactoring: SyncSession & MetadataSyncManager

A refactoring to improve separation of concerns in the metadata sync infrastructure, making it scalable for many entity services (Posts, Comments, Media, Users, Pages, Reader, etc.).

## Problem Statement

The current implementation has **concurrency helpers** (`begin_refresh`, `begin_fetch_next_page`, `complete_sync`, `complete_sync_with_error`) in `ListMetadataRepository`. These methods:

1. **Orchestrate multiple operations** (e.g., `begin_refresh` calls `get_or_create_and_increment_version` + `update_state_by_list_metadata_id`)
2. **Apply business logic** (e.g., checking if there are more pages to load)
3. **Manage state machine transitions**

This violates the repository pattern - repositories should be pure data access, not workflow orchestration.

Additionally, `PostService::fetch_metadata_persistent` has verbose error handling that would be duplicated in every future entity service:

```rust
// Current pattern - repeated at every step:
let result = self.fetch_posts_metadata(...).await.map_err(|e| {
    let _ = self.metadata_service.complete_sync_with_error(list_metadata_id, &e.to_string());
    e
})?;
```

---

## Goals

1. **Clean separation of concerns**:
   - Repository: SQL primitives only
   - Service: Domain operations with connection management
   - SyncManager: Workflow orchestration logic

2. **Eliminate error handling boilerplate** via RAII pattern

3. **Make sync pattern reusable** for future entity services (CommentService, MediaService, etc.)

4. **Keep it understandable** for developers with varying Rust experience

---

## Design Decision: Why RAII over Alternatives

We considered four options:

### Option A: SyncSession with RAII (CHOSEN)
A session struct with `Drop` that auto-completes with error if not explicitly completed.

### Option B: Higher-order function with async closure
```rust
metadata_service.with_sync(key, |ctx| async { ... }).await
```
**Rejected**: Async closures in Rust are painful (lifetime issues, `Pin<Box<dyn Future>>`).

### Option C: Trait for syncable services
```rust
trait SyncableEntityService { ... }
```
**Rejected**: Trait bounds + async are complex; harder for newcomers to extend.

### Option D: Keep it simple
Just move helpers to manager, let services duplicate orchestration.
**Rejected**: With many expected entity services, duplication becomes a maintenance burden.

### Why Option A wins:

1. **Idiomatic without being advanced**: RAII is fundamental Rust, but `SyncSession` is just a struct with `Drop`. No trait bounds, no async magic.

2. **Explicit control flow**: Each service's fetch method reads linearly. No hidden control flow.

3. **Automatic error safety**: The `?` operator works naturally - early returns trigger `Drop` cleanup.

4. **Entity-specific code fits naturally**: Just add between begin and complete.

5. **Easy to onboard**: "Create a session, do your work, complete the session."

6. **Scales through composition, not abstraction**: Each new service follows the same pattern without trait constraints.

---

## Architecture After Refactoring

```
┌─────────────────────────────────────────────────────────────────────┐
│ PostService / CommentService / MediaService / etc.                  │
│                                                                     │
│  pub async fn fetch_metadata_persistent(...) {                      │
│      let session = self.metadata_service.begin_sync(...)?;          │
│      let result = self.fetch_posts_metadata(...).await?;  // auto-cleanup on ?
│      self.metadata_service.store(&session, &result)?;     // auto-cleanup on ?
│      session.complete()?;                                           │
│  }                                                                  │
└─────────────────────────────────┬───────────────────────────────────┘
                                  │ uses
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│ MetadataService (wp_mobile/src/service/metadata.rs)                 │
│                                                                     │
│ - High-level API, owns WpApiCache + DbSite                          │
│ - Manages connection lifecycle via cache.execute()                  │
│ - Creates SyncSession, provides store/pagination helpers            │
│                                                                     │
│ pub fn begin_sync(...) -> Result<SyncSession, Error>                │
│ pub fn store(&self, session: &SyncSession, items: &[...]) -> ...    │
│ pub fn update_pagination(&self, session: &SyncSession, ...) -> ...  │
│                                                                     │
│ (Also: complete_sync, complete_sync_with_error - called by session) │
└─────────────────────────────────┬───────────────────────────────────┘
                                  │ calls
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│ MetadataSyncManager (wp_mobile/src/sync/metadata_sync_manager.rs)   │
│                                                                     │
│ - Stateless associated functions                                    │
│ - Takes QueryExecutor + DbSite + ListKey                            │
│ - Composes repository primitives into workflow operations           │
│                                                                     │
│ pub fn begin_refresh(executor, site, key, per_page) -> RefreshInfo  │
│ pub fn begin_fetch_next_page(executor, site, key) -> Option<...>    │
│ pub fn complete_sync(executor, list_metadata_id) -> ()              │
│ pub fn complete_sync_with_error(executor, id, msg) -> ()            │
└─────────────────────────────────┬───────────────────────────────────┘
                                  │ composes
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│ ListMetadataRepository (wp_mobile_cache/src/repository/)            │
│                                                                     │
│ - Pure SQL primitives                                               │
│ - get_or_create_and_increment_version (efficient UPSERT+RETURNING)  │
│ - update_state_by_list_metadata_id                                  │
│ - get_header, get_items, set_items, append_items, etc.              │
│                                                                     │
│ REMOVES: begin_refresh, begin_fetch_next_page, complete_sync, etc.  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## SyncSession Design

```rust
/// Represents an active sync operation with automatic error cleanup.
///
/// When a `SyncSession` is dropped without calling `complete()`, it
/// automatically marks the sync as failed. This ensures cleanup happens
/// even on panics or early `?` returns.
///
/// # Usage
///
/// ```rust
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

    /// Reference to service for cleanup (options discussed below)
    metadata_service: Arc<MetadataService>,

    /// Whether complete() was called
    completed: bool,
}

impl SyncSession {
    /// Get the list metadata ID for operations that need it
    pub fn list_metadata_id(&self) -> RowId {
        self.list_metadata_id
    }

    /// Get the version for stale detection
    pub fn version(&self) -> i64 {
        self.version
    }

    /// Mark sync as successfully completed.
    ///
    /// This sets state to Idle and prevents the Drop cleanup.
    pub fn complete(mut self) -> Result<(), WpServiceError> {
        self.completed = true;
        self.metadata_service.complete_sync_internal(self.list_metadata_id)
    }
}

impl Drop for SyncSession {
    fn drop(&mut self) {
        if !self.completed {
            // Log that we're doing automatic cleanup
            log::warn!(
                "SyncSession dropped without completion for list_metadata_id={}, marking as error",
                self.list_metadata_id.0
            );
            let _ = self.metadata_service.complete_sync_with_error_internal(
                self.list_metadata_id,
                "Sync session dropped without completion (likely due to error)",
            );
        }
    }
}
```

### Design Note: Service Reference in Session

The session needs to call back to the service for cleanup. Options:

1. **Arc<MetadataService>** (shown above): Works, but requires MetadataService to be wrapped in Arc.

2. **Weak<MetadataService>**: Avoids reference cycle, but upgrade() might fail.

3. **Callback closure**: `on_drop: Box<dyn FnOnce(RowId)>` - flexible but allocates.

4. **Return token, caller cleans up**: No RAII, loses automatic cleanup.

**Recommendation**: Use `Arc<MetadataService>` since `PostService` already stores it as `Arc<MetadataService>`. The session just clones the Arc.

---

## MetadataSyncManager Design

```rust
/// Manages sync workflow state transitions for list metadata.
///
/// This is a stateless helper that composes repository primitives
/// into workflow operations. It handles:
/// - Starting refresh operations (increment version, set fetching state)
/// - Starting load-more operations (validate pagination, set fetching state)
/// - Completing sync operations (set idle/error state)
///
/// This is intentionally at a low level (takes QueryExecutor) so that
/// MetadataService can call it within its cache.execute() blocks.
pub struct MetadataSyncManager;

impl MetadataSyncManager {
    /// Begin a refresh operation (fetch first page).
    ///
    /// Atomically:
    /// 1. Creates the list header if needed
    /// 2. Increments version (invalidates any in-flight load-more)
    /// 3. Sets state to FetchingFirstPage
    ///
    /// Returns info needed for the API call and session creation.
    pub fn begin_refresh(
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &ListKey,
        per_page: i64,
    ) -> Result<RefreshInfo, SqliteDbError> {
        log::debug!("MetadataSyncManager::begin_refresh: key={}", key);

        // Atomic: get or create header with incremented version
        let header_info = ListMetadataRepository::get_or_create_and_increment_version(
            executor, site, key, per_page
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
    /// Returns None if cannot load more, Some(info) otherwise.
    pub fn begin_fetch_next_page(
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &ListKey,
    ) -> Result<Option<FetchNextPageInfo>, SqliteDbError> {
        log::debug!("MetadataSyncManager::begin_fetch_next_page: key={}", key);

        let header = match ListMetadataRepository::get_header(executor, site, key)? {
            Some(h) => h,
            None => return Ok(None),
        };

        // Must have loaded at least one page
        if header.current_page == 0 {
            return Ok(None);
        }

        // Check if there are more pages
        if let Some(total_pages) = header.total_pages {
            if header.current_page >= total_pages {
                return Ok(None);
            }
        }

        let next_page = header.current_page + 1;

        // Update state
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
    pub fn complete_sync(
        executor: &impl QueryExecutor,
        list_metadata_id: RowId,
    ) -> Result<(), SqliteDbError> {
        log::debug!("MetadataSyncManager::complete_sync: id={}", list_metadata_id.0);
        ListMetadataRepository::update_state_by_list_metadata_id(
            executor, list_metadata_id, ListState::Idle, None
        )
    }

    /// Complete a sync operation with an error.
    pub fn complete_sync_with_error(
        executor: &impl QueryExecutor,
        list_metadata_id: RowId,
        error_message: &str,
    ) -> Result<(), SqliteDbError> {
        log::debug!(
            "MetadataSyncManager::complete_sync_with_error: id={}, error={}",
            list_metadata_id.0, error_message
        );
        ListMetadataRepository::update_state_by_list_metadata_id(
            executor, list_metadata_id, ListState::Error, Some(error_message)
        )
    }
}
```

---

## Updated MetadataService API

```rust
impl MetadataService {
    // ============================================================
    // Sync Session Management (NEW)
    // ============================================================

    /// Begin a sync operation and return a session for automatic cleanup.
    ///
    /// The returned session will automatically mark the sync as failed
    /// if dropped without calling `complete()`.
    pub fn begin_sync(
        self: &Arc<Self>,
        key: &ListKey,
        per_page: i64,
        is_first_page: bool,
    ) -> Result<SyncSession, WpServiceError> {
        let info = self.cache.execute(|conn| {
            if is_first_page {
                MetadataSyncManager::begin_refresh(conn, &self.db_site, key, per_page)
                    .map(|info| (info.list_metadata_id, info.version, info.per_page, true))
            } else {
                MetadataSyncManager::begin_fetch_next_page(conn, &self.db_site, key)
                    .map(|opt| opt.map(|info| (info.list_metadata_id, info.version, info.per_page, false)))
            }
        })?;

        match info {
            Some((list_metadata_id, version, per_page, is_first_page)) => {
                Ok(SyncSession {
                    list_metadata_id,
                    version,
                    per_page,
                    is_first_page,
                    metadata_service: Arc::clone(self),
                    completed: false,
                })
            }
            None => Err(WpServiceError::NoMorePages),
        }
    }

    /// Store items for a sync session.
    ///
    /// Replaces items if first page, appends if subsequent page.
    pub fn store(
        &self,
        session: &SyncSession,
        items: &[EntityMetadata],
    ) -> Result<(), WpServiceError> {
        if session.is_first_page {
            self.set_items_internal(session.list_metadata_id, items)
        } else {
            self.append_items_internal(session.list_metadata_id, items)
        }
    }

    /// Update pagination for a sync session.
    pub fn update_pagination(
        &self,
        session: &SyncSession,
        total_pages: Option<i64>,
        total_items: Option<i64>,
        current_page: i64,
    ) -> Result<(), WpServiceError> {
        // ... implementation
    }

    // Internal methods called by SyncSession
    fn complete_sync_internal(&self, list_metadata_id: RowId) -> Result<(), WpServiceError> {
        self.cache.execute(|conn| {
            MetadataSyncManager::complete_sync(conn, list_metadata_id)
        }).map_err(Into::into)
    }

    fn complete_sync_with_error_internal(
        &self,
        list_metadata_id: RowId,
        error_message: &str,
    ) -> Result<(), WpServiceError> {
        self.cache.execute(|conn| {
            MetadataSyncManager::complete_sync_with_error(conn, list_metadata_id, error_message)
        }).map_err(Into::into)
    }
}
```

---

## Updated PostService Usage

**Before** (current - verbose error handling):

```rust
pub async fn fetch_metadata_persistent(...) -> Result<SyncResult, FetchError> {
    let list_metadata_id = if is_first_page {
        match self.metadata_service.begin_refresh(key, per_page as i64) {
            Ok(info) => info.list_metadata_id,
            Err(e) => return Err(FetchError::Database { err_message: e.to_string() }),
        }
    } else {
        match self.metadata_service.begin_fetch_next_page(key) {
            Ok(Some(info)) => info.list_metadata_id,
            Ok(None) => return Err(FetchError::Database { err_message: "...".to_string() }),
            Err(e) => return Err(FetchError::Database { err_message: e.to_string() }),
        }
    };

    let result = match self.fetch_posts_metadata(...).await {
        Ok(result) => result,
        Err(e) => {
            let _ = self.metadata_service.complete_sync_with_error(list_metadata_id, &e.to_string());
            return Err(e);
        }
    };

    if let Err(e) = self.metadata_service.set_items(key, per_page as i64, &result.metadata) {
        let _ = self.metadata_service.complete_sync_with_error(list_metadata_id, &e.to_string());
        return Err(FetchError::Database { err_message: e.to_string() });
    }

    // ... more error handling at each step ...

    if let Err(e) = self.metadata_service.complete_sync(list_metadata_id) {
        return Err(FetchError::Database { err_message: e.to_string() });
    }

    Ok(sync_result)
}
```

**After** (clean with SyncSession):

```rust
pub async fn fetch_metadata_persistent(...) -> Result<SyncResult, FetchError> {
    // Begin sync - returns session with RAII cleanup
    let session = self.metadata_service.begin_sync(key, per_page as i64, is_first_page)?;

    // Any `?` from here will trigger Drop cleanup
    let result = self.fetch_posts_metadata(endpoint_type, filter, page, per_page).await?;

    self.metadata_service.store(&session, &result.metadata)?;
    self.metadata_service.update_pagination(
        &session,
        result.total_pages.map(|p| p as i64),
        result.total_items,
        page as i64,
    )?;

    // Entity-specific processing (not part of session)
    self.detect_and_mark_stale_posts(&result.metadata);

    // Explicit success
    session.complete()?;

    Ok(SyncResult { ... })
}
```

---

## Implementation Plan

### Phase 1: Create MetadataSyncManager (Repository Layer)

| Step | Description | Files |
|------|-------------|-------|
| 1.1 | Create `MetadataSyncManager` with `begin_refresh` | `wp_mobile/src/sync/metadata_sync_manager.rs` |
| 1.2 | Add `begin_fetch_next_page` | `wp_mobile/src/sync/metadata_sync_manager.rs` |
| 1.3 | Add `complete_sync`, `complete_sync_with_error` | `wp_mobile/src/sync/metadata_sync_manager.rs` |
| 1.4 | Add unit tests | `wp_mobile/src/sync/metadata_sync_manager.rs` |
| 1.5 | Export from `sync` module | `wp_mobile/src/sync/mod.rs` |

### Phase 2: Remove Helpers from Repository

| Step | Description | Files |
|------|-------------|-------|
| 2.1 | Remove `begin_refresh` from repository | `wp_mobile_cache/src/repository/list_metadata.rs` |
| 2.2 | Remove `begin_fetch_next_page` from repository | `wp_mobile_cache/src/repository/list_metadata.rs` |
| 2.3 | Remove `complete_sync`, `complete_sync_with_error` | `wp_mobile_cache/src/repository/list_metadata.rs` |
| 2.4 | Remove `RefreshInfo`, `FetchNextPageInfo` structs | `wp_mobile_cache/src/repository/list_metadata.rs` |
| 2.5 | Update exports | `wp_mobile_cache/src/lib.rs` |

### Phase 3: Create SyncSession

| Step | Description | Files |
|------|-------------|-------|
| 3.1 | Create `SyncSession` struct with `Drop` impl | `wp_mobile/src/sync/sync_session.rs` |
| 3.2 | Add `complete()` method | `wp_mobile/src/sync/sync_session.rs` |
| 3.3 | Add tests for RAII cleanup | `wp_mobile/src/sync/sync_session.rs` |
| 3.4 | Export from `sync` module | `wp_mobile/src/sync/mod.rs` |

### Phase 4: Update MetadataService

| Step | Description | Files |
|------|-------------|-------|
| 4.1 | Add `begin_sync` method returning `SyncSession` | `wp_mobile/src/service/metadata.rs` |
| 4.2 | Add `store(&SyncSession, ...)` helper | `wp_mobile/src/service/metadata.rs` |
| 4.3 | Add `update_pagination(&SyncSession, ...)` helper | `wp_mobile/src/service/metadata.rs` |
| 4.4 | Add internal completion methods | `wp_mobile/src/service/metadata.rs` |
| 4.5 | Keep backward-compatible methods during transition | `wp_mobile/src/service/metadata.rs` |

### Phase 5: Update PostService

| Step | Description | Files |
|------|-------------|-------|
| 5.1 | Refactor `fetch_metadata_persistent` to use `SyncSession` | `wp_mobile/src/service/posts.rs` |
| 5.2 | Refactor `fetch_and_store_metadata_persistent` if exists | `wp_mobile/src/service/posts.rs` |
| 5.3 | Remove old error handling boilerplate | `wp_mobile/src/service/posts.rs` |
| 5.4 | Update tests | `wp_mobile/src/service/posts.rs` |

### Phase 6: Cleanup

| Step | Description | Files |
|------|-------------|-------|
| 6.1 | Remove backward-compat methods from MetadataService | `wp_mobile/src/service/metadata.rs` |
| 6.2 | Update documentation | This file |
| 6.3 | Run full test suite | - |
| 6.4 | Run Kotlin example app to verify | - |

---

## Assumptions

1. **MetadataService is stored as Arc**: `PostService` already has `metadata_service: Arc<MetadataService>`, so passing `Arc` to `SyncSession` is natural.

2. **Drop can call cache.execute()**: The `Drop` impl needs to access the database. This should work since `Drop` isn't async and `cache.execute()` is sync.

3. **No panics during normal operation**: While `Drop` handles panics, we assume normal code paths use `?` for errors.

4. **Single sync per key at a time**: The version-based concurrency control handles overlapping syncs, but we assume typical usage is one sync per list key.

---

## Verification Checklist

After implementation, verify:

### Functional
- [ ] `PostService::fetch_metadata_persistent` works for first page
- [ ] `PostService::fetch_metadata_persistent` works for subsequent pages
- [ ] Errors during fetch correctly set state to Error
- [ ] Errors during store correctly set state to Error
- [ ] Successful sync sets state to Idle
- [ ] Kotlin example app works (refresh, load more, state indicators)

### RAII Behavior
- [ ] Early return with `?` triggers Drop cleanup
- [ ] Panic triggers Drop cleanup (test with `#[should_panic]`)
- [ ] Explicit `complete()` prevents Drop cleanup
- [ ] Drop logs a warning when cleaning up

### Architecture
- [ ] `ListMetadataRepository` has no workflow methods
- [ ] `MetadataSyncManager` is stateless (associated functions only)
- [ ] `SyncSession` is the only way to start a sync (old methods removed)

### Tests
- [ ] All existing tests pass
- [ ] New tests for `MetadataSyncManager`
- [ ] New tests for `SyncSession` RAII behavior
- [ ] New tests for `MetadataService::begin_sync`

---

## Future: CommentService Pattern

Once this refactoring is complete, a future `CommentService` would follow the same pattern:

```rust
impl CommentService {
    pub async fn fetch_comments_metadata_persistent(...) -> Result<SyncResult, FetchError> {
        let session = self.metadata_service.begin_sync(key, per_page, is_first_page)?;

        let result = self.fetch_comments_metadata(...).await?;

        self.metadata_service.store(&session, &result.metadata)?;
        self.metadata_service.update_pagination(&session, ...)?;

        // Comment-specific processing (if any)

        session.complete()?;
        Ok(...)
    }
}
```

The pattern is consistent, the error handling is automatic, and entity-specific logic fits naturally.

---

## Open Questions

1. **WpServiceError::NoMorePages**: Should `begin_sync` return a specific error for "no more pages" or should it return `None` like the current `begin_fetch_next_page`? Current design uses an error for simpler `?` usage.

2. **Logging level**: Should Drop cleanup be `warn!` or `debug!`? Currently using `warn!` since it indicates something went wrong.

3. **Session lifetime across await points**: The session must live across async calls. This should work fine since it's just a struct, but worth verifying.

---

## Changelog

| Date | Author | Changes |
|------|--------|---------|
| 2024-XX-XX | - | Initial design document |
