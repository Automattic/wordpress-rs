# MetadataService Orchestration Design (v2)

This document describes the refactoring of the metadata sync architecture to have `MetadataService` own the sync lifecycle, with entity services providing only the fetch implementation.

## Problem Statement

The current implementation has two issues:

1. **Mixed ownership**: `PostService` orchestrates the sync lifecycle (begin, fetch, store, complete) but this logic should be common across all entity services (Posts, Comments, Pages, etc.)

2. **Two parallel code paths**:
   - `fetch_and_store_metadata_persistent` uses `SyncSession`
   - `sync_post_list` uses legacy `_by_key` methods

   This is confusing and would lead to duplication as we add more entity services.

## Design Goals

1. **Single orchestration point**: `MetadataService` owns the sync lifecycle
2. **Entity services provide fetchers**: PostService just provides "how to fetch"
3. **Unit functions**: Each layer provides small, composable functions
4. **Optional bypass**: Entity services can bypass the orchestration if they have special needs
5. **No magic**: All calls are explicit and visible in the entity service code

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        PostService                               │
│                    (Entity-specific logic)                       │
│                                                                  │
│  - Provides fetch closure to MetadataService                     │
│  - Calls MetadataService::refresh() or load_more()               │
│  - Does entity-specific post-processing (detect_stale, etc.)     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ provides fetcher closure
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      MetadataService                             │
│                  (Lifecycle Orchestration)                       │
│                                                                  │
│  Orchestration (async, owns lifecycle):                          │
│    refresh(cache, site, key, per_page, fetcher)                  │
│    load_more(cache, site, key, fetcher)                          │
│                                                                  │
│  Internally does:                                                │
│    1. Set state to Fetching                                      │
│    2. Call fetcher                                               │
│    3. Store metadata (set or append)                             │
│    4. Update pagination                                          │
│    5. Set state to Idle (or Error on failure)                    │
│                                                                  │
│  Unit functions (for reads and special cases):                   │
│    get_state(), get_pagination(), get_entity_ids()               │
│    has_more_pages(), get_metadata()                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ calls
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   ListMetadataRepository                         │
│                     (SQL Primitives)                             │
│                                                                  │
│  All associated functions (stateless):                           │
│    get_header(), get_or_create()                                 │
│    get_items_by_list_key(), set_items_by_list_key()              │
│    append_items_by_list_key(), update_header_by_list_key()       │
│    update_state_by_list_metadata_id(), get_state_by_list_key()   │
│    get_or_create_and_increment_version()                         │
└─────────────────────────────────────────────────────────────────┘
```

## API Design

### PostService Usage (After)

```rust
impl PostService {
    pub async fn fetch_and_store_metadata_persistent(
        &self,
        key: &ListKey,
        endpoint_type: &PostEndpointType,
        filter: &PostListFilter,
        per_page: u32,
        is_first_page: bool,
    ) -> Result<MetadataFetchResult, FetchError> {

        // Note: page parameter removed - MetadataService determines the page
        let result = if is_first_page {
            MetadataService::refresh(
                &self.cache,
                &self.db_site,
                key,
                per_page,
                // Fetcher receives (page, per_page) from MetadataService
                |page, per_page| self.fetch_posts_metadata(endpoint_type, filter, page, per_page),
            ).await?
        } else {
            MetadataService::load_more(
                &self.cache,
                &self.db_site,
                key,
                // Fetcher receives (page, per_page) from MetadataService
                |page, per_page| self.fetch_posts_metadata(endpoint_type, filter, page, per_page),
            ).await?
        };

        // Entity-specific post-processing (not part of metadata layer)
        self.detect_and_mark_stale_posts(&result.metadata);

        Ok(result)
    }
}
```

### MetadataService Orchestration Functions

```rust
impl MetadataService {
    /// Refresh a list (fetch first page, replace existing data)
    ///
    /// Orchestrates the full sync lifecycle:
    /// 1. Increment version (invalidates in-flight load-more)
    /// 2. Set state to FetchingFirstPage
    /// 3. Call the fetcher with (page=1, per_page)
    /// 4. Store metadata (replacing existing)
    /// 5. Update pagination
    /// 6. Set state to Idle (or Error on failure)
    pub async fn refresh<F, Fut>(
        cache: &WpApiCache,
        site: &DbSite,
        key: &ListKey,
        per_page: u32,
        fetcher: F,
    ) -> Result<MetadataFetchResult, FetchError>
    where
        F: FnOnce(u32, u32) -> Fut,  // (page, per_page)
        Fut: Future<Output = Result<MetadataFetchResult, FetchError>>,
    {
        // 1. Begin refresh (increment version, set state)
        let info = cache.execute(|conn| {
            ListMetadataRepository::get_or_create_and_increment_version(conn, site, key, per_page)
        })?;

        cache.execute(|conn| {
            ListMetadataRepository::update_state_by_list_metadata_id(
                conn, info.row_id, ListState::FetchingFirstPage, None
            )
        })?;

        // 2. Call fetcher with page=1 (if this fails, set error state)
        let result = match fetcher(1, per_page).await {
            Ok(result) => result,
            Err(e) => {
                let _ = cache.execute(|conn| {
                    ListMetadataRepository::update_state_by_list_metadata_id(
                        conn, info.row_id, ListState::Error, Some(&e.to_string())
                    )
                });
                return Err(e);
            }
        };

        // 3. Store metadata
        let items = Self::to_item_inputs(&result.metadata);
        cache.execute(|conn| {
            ListMetadataRepository::set_items_by_list_metadata_id(conn, info.row_id, &items)
        })?;

        // 4. Update pagination
        cache.execute(|conn| {
            ListMetadataRepository::update_header_by_list_metadata_id(conn, info.row_id, &HeaderUpdate {
                total_pages: result.total_pages.map(|p| p as i64),
                total_items: result.total_items,
                current_page: 1,
                per_page: per_page as i64,
            })
        })?;

        // 5. Set state to Idle
        cache.execute(|conn| {
            ListMetadataRepository::update_state_by_list_metadata_id(
                conn, info.row_id, ListState::Idle, None
            )
        })?;

        Ok(result)
    }

    /// Load more (fetch next page, append to existing data)
    ///
    /// Similar to refresh but:
    /// - Gets per_page and next page number from existing state
    /// - Appends instead of replacing
    /// - Checks version hasn't changed (refresh didn't happen mid-load)
    pub async fn load_more<F, Fut>(
        cache: &WpApiCache,
        site: &DbSite,
        key: &ListKey,
        fetcher: F,
    ) -> Result<MetadataFetchResult, FetchError>
    where
        F: FnOnce(u32, u32) -> Fut,  // (page, per_page)
        Fut: Future<Output = Result<MetadataFetchResult, FetchError>>,
    {
        // 1. Get current state, determine next page
        let header = cache.execute(|conn| {
            ListMetadataRepository::get_header(conn, site, key)
        })?.ok_or_else(|| FetchError::Database {
            err_message: "Cannot load more: list not found".to_string()
        })?;

        let next_page = header.current_page + 1;
        let version = header.version;

        // Check if there are more pages
        if let Some(total) = header.total_pages {
            if header.current_page >= total {
                return Err(FetchError::Database {
                    err_message: "No more pages to load".to_string()
                });
            }
        }

        // 2. Set state to FetchingNextPage
        cache.execute(|conn| {
            ListMetadataRepository::update_state_by_list_metadata_id(
                conn, header.row_id, ListState::FetchingNextPage, None
            )
        })?;

        // 3. Call fetcher
        let result = match fetcher(next_page as u32, header.per_page as u32).await {
            Ok(result) => result,
            Err(e) => {
                let _ = cache.execute(|conn| {
                    ListMetadataRepository::update_state_by_list_metadata_id(
                        conn, header.row_id, ListState::Error, Some(&e.to_string())
                    )
                });
                return Err(e);
            }
        };

        // 4. Check version (refresh might have happened)
        let current_version = cache.execute(|conn| {
            ListMetadataRepository::get_version(conn, site, key)
        })?;

        if current_version != version {
            // A refresh happened, discard these results
            return Err(FetchError::Database {
                err_message: "List was refreshed during load more, discarding results".to_string()
            });
        }

        // 5. Append metadata
        let items = Self::to_item_inputs(&result.metadata);
        cache.execute(|conn| {
            ListMetadataRepository::append_items_by_list_metadata_id(conn, header.row_id, &items)
        })?;

        // 6. Update pagination
        cache.execute(|conn| {
            ListMetadataRepository::update_header_by_list_metadata_id(conn, header.row_id, &HeaderUpdate {
                total_pages: result.total_pages.map(|p| p as i64),
                total_items: result.total_items,
                current_page: next_page,
                per_page: header.per_page,
            })
        })?;

        // 7. Set state to Idle
        cache.execute(|conn| {
            ListMetadataRepository::update_state_by_list_metadata_id(
                conn, header.row_id, ListState::Idle, None
            )
        })?;

        Ok(result)
    }
}
```

## What Changes

### Remove

1. **SyncSession** (`wp_mobile/src/sync/sync_session.rs`) - No longer needed, lifecycle is internal
2. **MetadataSyncManager** (`wp_mobile/src/sync/metadata_sync_manager.rs`) - Merged into MetadataService
3. **Legacy `_by_key` methods** from MetadataService:
   - `complete_sync_by_key`
   - `complete_sync_with_error_by_key`
   - `begin_sync` (the SyncSession version)
   - `store_for_session`
   - `update_pagination_for_session`

### Keep (Unit Functions)

**MetadataService** (reads and special cases):
- `get_state()`
- `get_pagination()`
- `get_entity_ids()`
- `get_metadata()`
- `has_more_pages()`
- `get_version()`
- `delete_list()`

**ListMetadataRepository** (all SQL primitives - unchanged)

### Modify

**PostService**:
- `fetch_and_store_metadata_persistent` - Use new `MetadataService::refresh/load_more`
- `sync_post_list` - Use new pattern (or evaluate if still needed)

## Investigation Findings

### `sync_post_list` Analysis

Looking at `sync_post_list`, it does MORE than just metadata sync:

```
1. Set state to Fetching
2. Fetch metadata from API
3. Store metadata
4. Detect stale posts
5. Fetch FULL POST CONTENT for missing/stale posts  ← Entity-specific!
6. Update pagination
7. Set state to Idle
```

Steps 5 is entity-specific (fetching actual post content). The metadata layer shouldn't know about this.

**Recommendation**: `sync_post_list` should internally use `MetadataService::refresh/load_more` for the metadata part, then do the entity-specific full-post fetching separately:

```rust
pub async fn sync_post_list(...) -> Result<SyncResult, FetchError> {
    // Use MetadataService for metadata sync
    let metadata_result = if is_refresh {
        MetadataService::refresh(&self.cache, &self.db_site, key, per_page,
            || self.fetch_posts_metadata(...)).await?
    } else {
        MetadataService::load_more(&self.cache, &self.db_site, key,
            |page, per_page| self.fetch_posts_metadata(...)).await?
    };

    // Entity-specific: detect stale and fetch full posts
    self.detect_and_mark_stale_posts(&metadata_result.metadata);

    let ids_to_fetch = ...;  // filter missing/stale
    self.fetch_posts_by_ids(...).await?;

    Ok(SyncResult::new(...))
}
```

### Fetcher Signature Issue

For `load_more`, the fetcher needs to know WHAT page to fetch. Two options:

**Option A**: MetadataService passes (page, per_page) to fetcher
```rust
pub async fn load_more<F, Fut>(
    ...,
    fetcher: F,  // F: FnOnce(page: u32, per_page: u32) -> Fut
)
```

**Option B**: Fetcher is constructed with the info it needs
```rust
// PostService builds a closure that captures endpoint_type, filter
MetadataService::load_more(&cache, &site, &key,
    |page, per_page| self.fetch_posts_metadata(endpoint_type, filter, page, per_page)
).await?
```

**Recommendation**: Option A - pass (page, per_page) to fetcher. This is cleaner because:
- MetadataService knows the next page from DB state
- Fetcher doesn't need to know about MetadataService internals

### Current `fetch_and_store_metadata_persistent` vs `sync_post_list`

| Aspect | `fetch_and_store_metadata_persistent` | `sync_post_list` |
|--------|---------------------------------------|------------------|
| Fetches metadata | Yes | Yes |
| Stores metadata | Yes | Yes |
| Fetches full posts | No | Yes |
| Detects stale | Yes | Yes |
| Returns | MetadataFetchResult | SyncResult |

Both can use the new `refresh/load_more`, but `sync_post_list` adds the full-post-fetch step.

## Design Decisions

1. **Fetcher signature**: Fetcher receives `(page, per_page)` from MetadataService since it knows the next page from DB state.

2. **Associated functions (stateless)**: `refresh` and `load_more` are associated functions taking `&cache, &site` explicitly. More cumbersome but stateless and explicit - no hidden state.

3. **Error types**: Use distinct variants for orchestration errors:
   - `FetchError::NoMorePages` - when `load_more` is called but already at last page
   - `FetchError::VersionMismatch` - when refresh happened during load_more (stale results discarded)
   - Keep `FetchError::Api` for network errors
   - Keep `FetchError::Database` for DB errors

   Rationale: Distinct variants allow callers to handle these cases differently (e.g., UI might show "You're all caught up!" for NoMorePages vs retry for Api errors).

## Implementation Plan

| Phase | Task | Files |
|-------|------|-------|
| 1 | Add `refresh()` to MetadataService (with tests) | `metadata.rs` |
| 2 | Add `load_more()` to MetadataService (with tests) | `metadata.rs` |
| 3 | Update `fetch_and_store_metadata_persistent` to use `refresh/load_more` | `posts.rs` |
| 4 | Update `sync_post_list` to use `refresh/load_more` internally | `posts.rs` |
| 5 | Remove SyncSession | `sync_session.rs`, `sync/mod.rs` |
| 6 | Remove MetadataSyncManager (logic now in MetadataService) | `metadata_sync_manager.rs`, `sync/mod.rs` |
| 7 | Remove legacy methods from MetadataService | `metadata.rs` |
| 8 | Clean up unused imports and exports | various |

### Detailed Phase Breakdown

**Phase 1-2: Add orchestration methods**
- These are async methods that own the full lifecycle
- Error handling sets state to Error internally
- Return `MetadataFetchResult` on success

**Phase 3-4: Update PostService**
- Both methods become thin wrappers around `refresh/load_more`
- `sync_post_list` adds entity-specific full-post fetching after metadata sync

**Phase 5-7: Cleanup**
- Remove code that's no longer needed
- Should have no functional changes, just removal

## Diagrams

See `metadata_orchestration_flow.mmd` (and `.png`) for the visual flow.

## Verification

- [x] `PostService::fetch_and_store_metadata_persistent` uses `refresh/load_more`
- [x] No duplicate lifecycle code across entity services
- [x] SyncSession removed
- [x] MetadataSyncManager removed (merged into MetadataService as private helpers)
- [x] Legacy methods removed from MetadataService
- [x] All unit tests pass (65 tests)
- [ ] Kotlin example app works

## Implementation Notes

1. **Page parameter removed from API**: The `page` parameter was removed from
   `fetch_and_store_metadata_persistent`, `MetadataFetcher::fetch_metadata`, and
   `sync_post_list`. MetadataService now determines the page internally.

2. **MetadataSyncManager merged**: The workflow logic from MetadataSyncManager was
   inlined into MetadataService as private helper methods (`begin_refresh`,
   `begin_load_more`, `complete_sync`, `complete_sync_with_error`). This eliminates
   the dual-path issue where some calls went through MetadataSyncManager while
   others went directly to ListMetadataRepository.
