# MetadataService Implementation Plan

Implementation order: simple/low-level → complex/high-level. Each phase produces working, testable code.

## Phase 1: Database Foundation (wp_mobile_cache)

### 1.1 Add DbTable Variants
**File**: `wp_mobile_cache/src/lib.rs`

Add three new variants to `DbTable` enum:
- `ListMetadata`
- `ListMetadataItems`
- `ListMetadataState`

Update `table_name()` and `TryFrom<&str>` implementations.

**Commit**: "Add DbTable variants for list metadata tables"

### 1.2 Create Migration
**File**: `wp_mobile_cache/migrations/0007-create-list-metadata-tables.sql`

Create all three tables in one migration:
- `list_metadata` (header/pagination)
- `list_metadata_items` (items with rowid ordering)
- `list_metadata_state` (FK to list_metadata)

Add to `MIGRATION_QUERIES` array in `lib.rs`.

**Commit**: "Add migration for list metadata tables"

### 1.3 Create Database Types
**File**: `wp_mobile_cache/src/db_types/list_metadata.rs`

Define:
- `ListMetadataColumn` enum (column indices)
- `DbListMetadata` struct (header row)
- `ListMetadataItemColumn` enum
- `DbListMetadataItem` struct (item row)
- `ListMetadataStateColumn` enum
- `DbListMetadataState` struct (state row)
- `ListState` enum (idle, fetching_first_page, fetching_next_page, error)

Export from `db_types/mod.rs`.

**Commit**: "Add database types for list metadata"

### 1.4 Create Repository - Basic Operations
**File**: `wp_mobile_cache/src/repository/list_metadata.rs`

Implement `ListMetadataRepository` with:
- `get_or_create(db_site, key)` → returns header rowid
- `get_header(db_site, key)` → Option<DbListMetadata>
- `get_items(db_site, key)` → Vec<DbListMetadataItem> (ORDER BY rowid)
- `get_state(list_metadata_id)` → Option<DbListMetadataState>

Export from `repository/mod.rs`.

**Commit**: "Add list metadata repository with read operations"

### 1.5 Repository - Write Operations
**File**: `wp_mobile_cache/src/repository/list_metadata.rs`

Add write methods:
- `set_items(db_site, key, items)` → DELETE + INSERT (for refresh)
- `append_items(db_site, key, items)` → INSERT (for load more)
- `update_header(db_site, key, updates)` → UPDATE pagination info
- `update_state(list_metadata_id, state, error_msg)` → UPSERT state
- `increment_version(db_site, key)` → bump version, return new value

**Commit**: "Add list metadata repository write operations"

### 1.6 Repository - Concurrency Support
**File**: `wp_mobile_cache/src/repository/list_metadata.rs`

Add:
- `begin_fetch_next_page(db_site, key)` → updates state, returns FetchNextPageInfo
- `begin_refresh(db_site, key)` → updates state, increments version, returns info
- `check_version(db_site, key, expected)` → bool for stale check

**Commit**: "Add list metadata repository concurrency helpers"

---

## Phase 2: MetadataService (wp_mobile)

### 2.1 Create MetadataService Struct
**File**: `wp_mobile/src/service/metadata.rs`

Create `MetadataService`:
```rust
pub struct MetadataService {
    cache: Arc<WpApiCache>,
}
```

Basic methods wrapping repository:
- `new(cache)`
- `get_items(db_site, key)` → Vec<EntityMetadata>
- `get_pagination(db_site, key)` → Option<PaginationInfo>

Export from `service/mod.rs`.

**Commit**: "Add MetadataService with basic read operations"

### 2.2 MetadataService - Write Operations
**File**: `wp_mobile/src/service/metadata.rs`

Add:
- `store_items(db_site, key, items, is_first_page)`
- `update_pagination(db_site, key, total_pages, total_items, current_page)`

**Commit**: "Add MetadataService write operations"

### 2.3 MetadataService - State Management
**File**: `wp_mobile/src/service/metadata.rs`

Add:
- `begin_refresh(db_site, key)` → Result<RefreshInfo>
- `begin_load_next_page(db_site, key)` → Result<Option<LoadNextPageInfo>>
- `complete_sync(db_site, key, success)` → updates state to idle/error
- `get_sync_state(db_site, key)` → ListState

**Commit**: "Add MetadataService state management"

### 2.4 Implement Reader Trait
**File**: `wp_mobile/src/service/metadata.rs`

Implement `ListMetadataReader` trait for MetadataService (or a reader wrapper):
- `get(key)` → Option<Vec<EntityMetadata>>

This allows MetadataCollection to read from DB via the existing trait.

**Commit**: "Implement ListMetadataReader for MetadataService"

---

## Phase 3: Integration

### 3.1 Update MetadataCollection - Closure Pattern
**File**: `wp_mobile/src/sync/metadata_collection.rs`

Replace `fetcher: F` with sync callback closure:
```rust
sync_callback: Box<dyn Fn(u32, bool) -> BoxFuture<Result<SyncResult, FetchError>> + Send + Sync>
```

Update `refresh()` and `load_next_page()` to use callback.
Keep `items()`, `is_relevant_update()`, pagination methods.

**Commit**: "Refactor MetadataCollection to use sync callback"

### 3.2 Update PostService - Use MetadataService
**File**: `wp_mobile/src/service/posts.rs`

Changes:
- Add `metadata_service: Arc<MetadataService>` field
- Remove `metadata_store: Arc<ListMetadataStore>` field
- Update `metadata_reader()` to return MetadataService's reader
- Create `sync_post_list(key, filter, page, is_refresh)` method that orchestrates:
  1. Update state via MetadataService
  2. Fetch metadata from API
  3. Detect staleness
  4. Fetch missing/stale posts
  5. Store items via MetadataService
  6. Update state to idle

**Commit**: "Integrate MetadataService into PostService"

### 3.3 Update Collection Creation
**File**: `wp_mobile/src/service/posts.rs`

Update `create_post_metadata_collection_with_edit_context`:
- Create sync callback that calls `sync_post_list`
- Pass callback to MetadataCollection
- Remove fetcher creation

**Commit**: "Update post metadata collection to use sync callback"

### 3.4 Remove Old Components
**Files**:
- Delete `wp_mobile/src/sync/list_metadata_store.rs`
- Delete `wp_mobile/src/sync/post_metadata_fetcher.rs`
- Update `wp_mobile/src/sync/mod.rs` exports
- Remove `MetadataFetcher` trait if no longer needed

**Commit**: "Remove deprecated in-memory metadata store and fetcher"

---

## Phase 4: Observer Split

### 4.1 Split is_relevant_update
**File**: `wp_mobile/src/sync/metadata_collection.rs`

Replace single `is_relevant_update` with:
- `is_relevant_data_update(hook)` → checks ListMetadataItems + entity tables
- `is_relevant_state_update(hook)` → checks ListMetadataState

Need to store `list_metadata_id` or derive it for state matching.

**Commit**: "Split is_relevant_update into data and state checks"

### 4.2 Update Kotlin Wrapper
**File**: `native/kotlin/api/kotlin/src/main/kotlin/rs/wordpress/cache/kotlin/ObservableMetadataCollection.kt`

Changes:
- Split `observers` into `dataObservers` and `stateObservers`
- Add `addDataObserver()`, `addStateObserver()`, `removeDataObserver()`, `removeStateObserver()`
- Keep `addObserver()` as convenience (adds to both)
- Update `notifyIfRelevant()` to call appropriate observer lists

**Commit**: "Split ObservableMetadataCollection observers for data vs state"

### 4.3 Add State Query Method
**Files**:
- `wp_mobile/src/collection/post_metadata_collection.rs`
- Kotlin wrapper

Add method to query current sync state:
- `syncState()` → ListState (idle, fetching_first_page, etc.)

Useful for UI to show loading indicators.

**Commit**: "Add syncState query to metadata collections"

---

## Phase 5: Testing & Cleanup

### 5.1 Add Repository Tests
**File**: `wp_mobile_cache/src/repository/list_metadata.rs`

Unit tests for:
- Basic CRUD operations
- set_items replaces, append_items appends
- Version incrementing
- State transitions
- Concurrency helpers

**Commit**: "Add list metadata repository tests"

### 5.2 Add Service Tests
**File**: `wp_mobile/src/service/metadata.rs`

Unit tests for MetadataService operations.

**Commit**: "Add MetadataService tests"

### 5.3 Update Example App
**File**: Kotlin example app

Update to demonstrate:
- Data observers for list content
- State observers for loading indicator
- Pull-to-refresh with proper state transitions

**Commit**: "Update example app for split observers"

---

## Dependency Order Summary

```
Phase 1.1 (DbTable)
    ↓
Phase 1.2 (Migration)
    ↓
Phase 1.3 (DB Types)
    ↓
Phase 1.4-1.6 (Repository)
    ↓
Phase 2.1-2.4 (MetadataService)
    ↓
Phase 3.1 (Collection refactor) ←── can be done in parallel with 3.2
    ↓
Phase 3.2-3.3 (PostService integration)
    ↓
Phase 3.4 (Cleanup)
    ↓
Phase 4.1-4.3 (Observer split)
    ↓
Phase 5 (Testing)
```

## Risk Areas

1. **Migration on existing DBs**: Test migration on DB with existing data
2. **Async closure lifetime**: The sync callback closure captures Arc references - verify no lifetime issues
3. **Observer notification timing**: Ensure DB updates trigger hooks correctly for new tables
4. **UniFFI exports**: New types (ListState, etc.) need proper uniffi annotations

## Verification Checkpoints

After each phase, verify:
- `cargo build` succeeds
- `cargo test --lib` passes
- `cargo clippy` has no warnings

After Phase 3:
- Kotlin example app builds and runs
- Pull-to-refresh works
- Pagination works

After Phase 4:
- State observers fire on sync start/end
- Data observers fire on list content change
- No duplicate notifications
