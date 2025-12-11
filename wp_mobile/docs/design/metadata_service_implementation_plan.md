# MetadataService Implementation Plan

Implementation order: simple/low-level → complex/high-level. Each phase produces working, testable code.

## Phase 1: Database Foundation (wp_mobile_cache) ✅ COMPLETE

### 1.1 Add DbTable Variants ✅
**File**: `wp_mobile_cache/src/lib.rs`

Add three new variants to `DbTable` enum:
- `ListMetadata`
- `ListMetadataItems`
- `ListMetadataState`

Update `table_name()` and `TryFrom<&str>` implementations.

**Commit**: `3c95dfb4` - "Add database foundation for MetadataService (Phase 1)"

### 1.2 Create Migration ✅
**File**: `wp_mobile_cache/migrations/0007-create-list-metadata-tables.sql`

Create all three tables in one migration:
- `list_metadata` (header/pagination)
- `list_metadata_items` (items with rowid ordering)
- `list_metadata_state` (FK to list_metadata)

Add to `MIGRATION_QUERIES` array in `lib.rs`.

**Commit**: (included in 1.1 commit)

### 1.3 Create Database Types ✅
**Files**:
- `wp_mobile_cache/src/list_metadata.rs` (structs and ListState enum)
- `wp_mobile_cache/src/db_types/db_list_metadata.rs` (column enums and from_row)

Define:
- `DbListMetadata` struct (header row)
- `DbListMetadataItem` struct (item row)
- `DbListMetadataState` struct (state row)
- `ListState` enum (idle, fetching_first_page, fetching_next_page, error)
- Column enums for each table

**Commit**: (included in 1.1 commit)

### 1.4 Create Repository - Basic Operations ✅
**File**: `wp_mobile_cache/src/repository/list_metadata.rs`

Implement `ListMetadataRepository` with:
- `get_or_create(db_site, key)` → returns header rowid
- `get_header(db_site, key)` → Option<DbListMetadata>
- `get_items(db_site, key)` → Vec<DbListMetadataItem> (ORDER BY rowid)
- `get_state(list_metadata_id)` → Option<DbListMetadataState>
- `get_state_by_key(db_site, key)` → ListState
- `get_version(db_site, key)` → i64
- `check_version(db_site, key, expected)` → bool
- `get_item_count(db_site, key)` → i64

**Commit**: (included in 1.1 commit)

### 1.5 Repository - Write Operations ✅
**File**: `wp_mobile_cache/src/repository/list_metadata.rs`

Add write methods:
- `set_items(db_site, key, items)` → DELETE + INSERT (for refresh)
- `append_items(db_site, key, items)` → INSERT (for load more)
- `update_header(db_site, key, updates)` → UPDATE pagination info
- `update_state(list_metadata_id, state, error_msg)` → UPSERT state
- `update_state_by_key(db_site, key, state, error_msg)` → convenience wrapper
- `increment_version(db_site, key)` → bump version, return new value
- `delete_list(db_site, key)` → delete all data for a list

**Commit**: (included in 1.1 commit)

### 1.6 Repository - Concurrency Support ✅
**File**: `wp_mobile_cache/src/repository/list_metadata.rs`

Add:
- `begin_refresh(db_site, key)` → updates state, increments version, returns RefreshInfo
- `begin_fetch_next_page(db_site, key)` → updates state, returns FetchNextPageInfo
- `complete_sync(list_metadata_id)` → sets state to Idle
- `complete_sync_with_error(list_metadata_id, error_msg)` → sets state to Error

**Commit**: `e484f791` - "Add list metadata repository concurrency helpers"

---

## Phase 2: MetadataService (wp_mobile) ✅ COMPLETE

### 2.1-2.4 Create MetadataService ✅
**File**: `wp_mobile/src/service/metadata.rs`

Created `MetadataService` with all operations in a single implementation:

**Read operations:**
- `get_entity_ids(key)` → Vec<i64>
- `get_metadata(key)` → Option<Vec<EntityMetadata>>
- `get_state(key)` → ListState
- `get_pagination(key)` → Option<ListPaginationInfo>
- `has_more_pages(key)` → bool
- `get_version(key)` → i64
- `check_version(key, expected)` → bool

**Write operations:**
- `set_items(key, metadata)` → replace items
- `append_items(key, metadata)` → append items
- `update_pagination(key, total_pages, total_items, current_page, per_page)`
- `delete_list(key)`

**State management:**
- `set_state(key, state, error_message)`
- `begin_refresh(key)` → RefreshInfo
- `begin_fetch_next_page(key)` → Option<FetchNextPageInfo>
- `complete_sync(key)`
- `complete_sync_with_error(key, error_message)`

**Trait implementation:**
- `ListMetadataReader` trait implemented for MetadataService

**Commit**: `3c85514b` - "Add MetadataService for database-backed list metadata"

---

## Phase 3: Integration ✅ PARTIAL COMPLETE

### 3.1 Update MetadataCollection - Closure Pattern
**Status**: NOT STARTED (deferred)

The existing MetadataCollection still uses the `MetadataFetcher` trait pattern.
This refactoring is optional since `sync_post_list` provides the new pattern.

### 3.2 Update PostService - Use MetadataService ✅
**File**: `wp_mobile/src/service/posts.rs`

Changes:
- Added `metadata_service: Arc<MetadataService>` field
- Added `persistent_metadata_reader()` → Arc<MetadataService>
- Added `metadata_service()` → Arc<MetadataService>
- Added `sync_post_list(key, filter, page, per_page, is_refresh)` method that orchestrates:
  1. Update state via MetadataService (FetchingFirstPage/FetchingNextPage)
  2. Fetch metadata from API
  3. Store items via MetadataService
  4. Detect staleness via modified_gmt comparison
  5. Fetch missing/stale posts
  6. Update pagination info
  7. Set state back to Idle (or Error on failure)

Also extended `SyncResult` with `current_page` and `total_pages` fields.

**Commit**: `5c83b435` - "Integrate MetadataService into PostService"

### 3.3 Update Collection Creation ✅
**Status**: COMPLETE

Updated `create_post_metadata_collection_with_edit_context` to use persistent (database-backed) storage:

Changes:
- Added `fetch_and_store_metadata_persistent()` method to PostService (stores to MetadataService)
- Created `PersistentPostMetadataFetcherWithEditContext` that uses the new method
- Updated collection to use `persistent_metadata_reader()` instead of `metadata_reader()`
- Added `DbTable::ListMetadataItems` to relevant_tables for data update notifications
- Updated `PostMetadataCollectionWithEditContext` to use the persistent fetcher type

### 3.4 Remove Old Components
**Status**: NOT STARTED

The in-memory `ListMetadataStore` is preserved for backwards compatibility.
Can be removed once all callers migrate to persistent service.

---

## Phase 4: Observer Split

### 4.1 Split is_relevant_update
**Status**: NOT STARTED
**File**: `wp_mobile/src/sync/metadata_collection.rs`

Replace single `is_relevant_update` with:
- `is_relevant_data_update(hook)` → checks ListMetadataItems + entity tables
- `is_relevant_state_update(hook)` → checks ListMetadataState

Need to store `list_metadata_id` or derive it for state matching.

**Commit**: "Split is_relevant_update into data and state checks"

### 4.2 Update Kotlin Wrapper
**Status**: NOT STARTED
**File**: `native/kotlin/api/kotlin/src/main/kotlin/rs/wordpress/cache/kotlin/ObservableMetadataCollection.kt`

Changes:
- Split `observers` into `dataObservers` and `stateObservers`
- Add `addDataObserver()`, `addStateObserver()`, `removeDataObserver()`, `removeStateObserver()`
- Keep `addObserver()` as convenience (adds to both)
- Update `notifyIfRelevant()` to call appropriate observer lists

**Commit**: "Split ObservableMetadataCollection observers for data vs state"

### 4.3 Add State Query Method
**Status**: NOT STARTED
**Files**:
- `wp_mobile/src/collection/post_metadata_collection.rs`
- Kotlin wrapper

Add method to query current sync state:
- `syncState()` → ListState (idle, fetching_first_page, etc.)

Useful for UI to show loading indicators.

**Commit**: "Add syncState query to metadata collections"

---

## Phase 5: Testing & Cleanup

### 5.1 Add Repository Tests ✅
**File**: `wp_mobile_cache/src/repository/list_metadata.rs`

Unit tests for:
- Basic CRUD operations
- set_items replaces, append_items appends
- Version incrementing
- State transitions
- Concurrency helpers

**Test count**: 31 tests in list_metadata repository

### 5.2 Add Service Tests ✅
**File**: `wp_mobile/src/service/metadata.rs`

Unit tests for MetadataService operations.

**Test count**: 15 tests in MetadataService

### 5.3 Update Example App
**Status**: NOT STARTED
**File**: Kotlin example app

Update to demonstrate:
- Data observers for list content
- State observers for loading indicator
- Pull-to-refresh with proper state transitions

**Commit**: "Update example app for split observers"

---

## Current Status Summary

| Phase | Status | Commits |
|-------|--------|---------|
| 1.1-1.5 | ✅ Complete | `3c95dfb4` |
| 1.6 | ✅ Complete | `e484f791` |
| 2.1-2.4 | ✅ Complete | `3c85514b` |
| 3.1 | ⏸️ Deferred | - |
| 3.2 | ✅ Complete | `5c83b435` |
| 3.3 | ✅ Complete | - |
| 3.4 | 🔲 Not Started | - |
| 4.1-4.3 | 🔲 Not Started | - |
| 5.1-5.2 | ✅ Complete | (inline) |
| 5.3 | 🔲 Not Started | - |

## Dependency Order Summary

```
Phase 1.1 (DbTable) ✅
    ↓
Phase 1.2 (Migration) ✅
    ↓
Phase 1.3 (DB Types) ✅
    ↓
Phase 1.4-1.6 (Repository) ✅
    ↓
Phase 2.1-2.4 (MetadataService) ✅
    ↓
Phase 3.1 (Collection refactor) ⏸️ deferred
    ↓
Phase 3.2-3.3 (PostService integration) ✅
    ↓
Phase 3.4 (Cleanup) 🔲
    ↓
Phase 4.1-4.3 (Observer split) 🔲
    ↓
Phase 5 (Testing) ✅ partial
```

## Risk Areas

1. **Migration on existing DBs**: Test migration on DB with existing data
2. **Async closure lifetime**: The sync callback closure captures Arc references - verify no lifetime issues
3. **Observer notification timing**: Ensure DB updates trigger hooks correctly for new tables
4. **UniFFI exports**: New types (ListState, etc.) need proper uniffi annotations

## Verification Checkpoints

After each phase, verify:
- `cargo build` succeeds ✅
- `cargo test --lib` passes ✅ (112 tests in wp_mobile_cache, 60 tests in wp_mobile)
- `cargo clippy` has no warnings ✅

After Phase 3:
- Kotlin example app builds and runs
- Pull-to-refresh works
- Pagination works

After Phase 4:
- State observers fire on sync start/end
- Data observers fire on list content change
- No duplicate notifications
