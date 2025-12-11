# MetadataService Design

This document captures the design decisions for moving list metadata from in-memory KV store to database tables, introducing MetadataService, and refactoring the sync architecture.

## Motivation

1. **No observer pattern for in-memory KV store** - Currently relies on Posts table updates to trigger UI refresh, which is fragile (e.g., if a post is removed from a list by status change, metadata changes but no observer fires)
2. **No persistence between launches** - List structure is lost on app restart
3. **Cleaner architecture** - Separate concerns: MetadataService for list management, PostService for entity-specific operations

## Database Schema

Three new tables to replace the in-memory `ListMetadataStore`:

```sql
-- Table 1: List header/pagination info
CREATE TABLE `list_metadata` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `db_site_id` INTEGER NOT NULL,
  `key` TEXT NOT NULL,              -- e.g., "edit:posts:publish"
  `total_pages` INTEGER,
  `total_items` INTEGER,
  `current_page` INTEGER NOT NULL DEFAULT 0,
  `per_page` INTEGER NOT NULL DEFAULT 20,
  `last_first_page_fetched_at` TEXT,
  `last_updated_at` TEXT,
  `version` INTEGER NOT NULL DEFAULT 0,

  FOREIGN KEY (db_site_id) REFERENCES db_sites(id) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_list_metadata_unique_key ON list_metadata(db_site_id, key);

-- Table 2: List items (rowid = insertion order = display order)
CREATE TABLE `list_metadata_items` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `db_site_id` INTEGER NOT NULL,
  `key` TEXT NOT NULL,
  `entity_id` INTEGER NOT NULL,     -- post/comment/etc ID
  `modified_gmt` TEXT,              -- nullable for entities without it

  FOREIGN KEY (db_site_id) REFERENCES db_sites(id) ON DELETE CASCADE
) STRICT;

CREATE INDEX idx_list_metadata_items_key ON list_metadata_items(db_site_id, key);
CREATE INDEX idx_list_metadata_items_entity ON list_metadata_items(db_site_id, entity_id);

-- Table 3: Sync state (FK to list_metadata, not duplicating key)
CREATE TABLE `list_metadata_state` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `list_metadata_id` INTEGER NOT NULL,
  `state` TEXT NOT NULL DEFAULT 'idle',  -- idle, fetching_first_page, fetching_next_page, error
  `error_message` TEXT,
  `updated_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

  FOREIGN KEY (list_metadata_id) REFERENCES list_metadata(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_list_metadata_state_unique ON list_metadata_state(list_metadata_id);
```

### Design Decisions

- **rowid for ordering**: Items are inserted in order, `ORDER BY rowid` gives correct sequence. No explicit position column needed.
- **db_site_id**: Follows existing pattern, allows querying all lists for a site.
- **key without embedded site_id**: Site is explicit in `db_site_id`, key is just the filter part (e.g., "edit:posts:publish").
- **version field**: Incremented on page 1 refresh. Used to detect stale concurrent operations (e.g., "load page 5" started before "pull to refresh" but finishes after).
- **State as separate table**: Different observers for data vs state changes. State changes (idle → fetching) don't need to trigger list reload.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      MetadataService                             │
├─────────────────────────────────────────────────────────────────┤
│ Owns:                                                           │
│ - list_metadata table (pagination, version)                     │
│ - list_metadata_items table (entity_id, modified_gmt, order)    │
│ - list_metadata_state table (idle/fetching/error)               │
│                                                                 │
│ Provides:                                                       │
│ - store_list(key, items, is_first_page) → stores items          │
│ - get_list_items(key) → reads items                             │
│ - update_state(key, state) → updates sync state                 │
│ - get_or_create_list_metadata(key) → ensures header exists      │
│ - check_version(key, expected) → for concurrency control        │
│ - Readers for MetadataCollection to use                         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ uses
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        PostService                               │
├─────────────────────────────────────────────────────────────────┤
│ Owns:                                                           │
│ - EntityStateStore (Cached/Stale/Missing per entity)            │
│ - Posts table operations                                        │
│                                                                 │
│ Provides:                                                       │
│ - create_post_metadata_collection() → creates collection        │
│ - sync_post_list(key, filter, page) → orchestrates sync         │
│   1. Fetch metadata from API                                    │
│   2. Check staleness (compare with posts table)                 │
│   3. Fetch missing/stale posts                                  │
│   4. Store list via MetadataService                             │
│ - Key generation (knows about filters)                          │
└─────────────────────────────────────────────────────────────────┘
```

### Two Types of State

1. **Entity state** (Cached, Stale, Missing, Fetching, Failed) - per entity, owned by PostService
2. **List state** (idle, fetching_first_page, fetching_next_page, error) - per list, owned by MetadataService

These serve different purposes:
- Entity state: "Is this post's data fresh?"
- List state: "Is this list currently syncing?"

## MetadataCollection Changes

MetadataCollection keeps convenience methods (`refresh()`, `load_next_page()`) but uses a closure pattern (like `StatelessCollection.load_data`) instead of owning a fetcher that references PostService:

```rust
struct MetadataCollection {
    key: String,
    db_site_id: RowId,

    // Readers from MetadataService (DB-backed)
    metadata_reader: Arc<dyn ListMetadataReader>,
    state_reader: Arc<dyn EntityStateReader>,  // Entity state from PostService

    // Tables to monitor for data updates
    relevant_data_tables: Vec<DbTable>,

    // Callback to trigger sync (provided by PostService)
    sync_callback: Box<dyn Fn(u32, bool) -> BoxFuture<Result<SyncResult, FetchError>> + Send + Sync>,
}

impl MetadataCollection {
    pub async fn refresh(&self) -> Result<SyncResult, FetchError> {
        (self.sync_callback)(1, true).await
    }

    pub async fn load_next_page(&self) -> Result<SyncResult, FetchError> {
        let next_page = self.current_page() + 1;
        (self.sync_callback)(next_page, false).await
    }

    pub fn items(&self) -> Vec<CollectionItem> {
        // Reads from MetadataService tables
    }

    /// Check if update affects list data
    /// Includes: list_metadata_items (structure) + Posts table (content)
    pub fn is_relevant_data_update(&self, hook: &UpdateHook) -> bool {
        self.relevant_data_tables.contains(&hook.table)
            || (hook.table == DbTable::ListMetadataItems && /* key matches */)
    }

    /// Check if update affects sync state
    pub fn is_relevant_state_update(&self, hook: &UpdateHook) -> bool {
        hook.table == DbTable::ListMetadataState && /* list_metadata_id matches */
    }
}
```

## Kotlin ObservableMetadataCollection Changes

Split observers for data vs state:

```kotlin
class ObservableMetadataCollection(
    private val collection: PostMetadataCollectionWithEditContext
) : AutoCloseable {
    private val dataObservers = CopyOnWriteArrayList<() -> Unit>()
    private val stateObservers = CopyOnWriteArrayList<() -> Unit>()

    fun addDataObserver(observer: () -> Unit) { dataObservers.add(observer) }
    fun addStateObserver(observer: () -> Unit) { stateObservers.add(observer) }

    // Convenience: observe both
    fun addObserver(observer: () -> Unit) {
        addDataObserver(observer)
        addStateObserver(observer)
    }

    fun removeDataObserver(observer: () -> Unit) { dataObservers.remove(observer) }
    fun removeStateObserver(observer: () -> Unit) { stateObservers.remove(observer) }

    internal fun notifyIfRelevant(hook: UpdateHook) {
        if (collection.isRelevantDataUpdate(hook)) {
            dataObservers.forEach { it() }
        }
        if (collection.isRelevantStateUpdate(hook)) {
            stateObservers.forEach { it() }
        }
    }

    // ... rest unchanged
}
```

**UI usage:**
```kotlin
// List content observes data changes
observableCollection.addDataObserver {
    items = observableCollection.loadItems()
}

// Pull-to-refresh indicator observes state changes
observableCollection.addStateObserver {
    isRefreshing = observableCollection.syncState() == SyncState.FETCHING_FIRST_PAGE
}
```

## Sync Flow

```
User triggers refresh
        │
        ▼
MetadataCollection.refresh()
        │
        ▼ (calls sync_callback)
PostService.sync_post_list(key, filter, page=1, is_refresh=true)
        │
        ├─► MetadataService.update_state(key, FETCHING_FIRST_PAGE)
        │   └─► DB update → state observers notified → UI shows spinner
        │
        ├─► PostService.fetch_posts_metadata(filter, page=1)
        │   └─► API call → returns [id, modified_gmt] list
        │
        ├─► PostService.detect_and_mark_stale_posts(metadata)
        │   └─► Compare with Posts table, mark stale in EntityStateStore
        │
        ├─► PostService.fetch_posts_by_ids(missing + stale IDs)
        │   └─► API call → upsert to Posts table → data observers notified
        │
        ├─► MetadataService.store_list(key, metadata, is_first_page=true)
        │   └─► DELETE + INSERT to list_metadata_items
        │   └─► Bump version in list_metadata
        │   └─► DB update → data observers notified → UI reloads list
        │
        └─► MetadataService.update_state(key, IDLE)
            └─► DB update → state observers notified → UI hides spinner
```

## Version-based Concurrency Control

Scenario:
1. User has loaded pages 1-4, current_page=4
2. User triggers "load page 5" (async)
3. User pulls to refresh before page 5 returns
4. Refresh completes: version bumped 5→6, list replaced with page 1
5. "Load page 5" completes with stale version=5

Solution:
```rust
// When starting load_next_page, capture current version
let version_at_start = metadata_service.get_version(key);

// ... async fetch ...

// Before storing, check version hasn't changed
if !metadata_service.check_version(key, version_at_start) {
    // Version changed (refresh happened), discard stale results
    return Ok(SyncResult::discarded());
}

// Version matches, safe to append
metadata_service.store_list(key, metadata, is_first_page=false);
```

## Files to Create/Modify

### New Files
- `wp_mobile_cache/migrations/0007-create-list-metadata-tables.sql`
- `wp_mobile_cache/src/repository/list_metadata.rs`
- `wp_mobile_cache/src/db_types/list_metadata.rs`
- `wp_mobile/src/service/metadata.rs` (MetadataService)

### Modified Files
- `wp_mobile_cache/src/lib.rs` - Add DbTable variants, exports
- `wp_mobile/src/service/posts.rs` - Use MetadataService, add sync_post_list
- `wp_mobile/src/sync/metadata_collection.rs` - DB-backed readers, split is_relevant_update
- `wp_mobile/src/sync/mod.rs` - Remove in-memory stores, update exports
- `native/kotlin/.../ObservableMetadataCollection.kt` - Split observers

### Files to Remove
- `wp_mobile/src/sync/list_metadata_store.rs` (replaced by DB)
- `wp_mobile/src/sync/post_metadata_fetcher.rs` (replaced by closure pattern)

## Implementation Notes

### Repository Pattern

The database implementation lives in the `wp_mobile_cache` crate and uses the repository pattern. We need to create:
- `wp_mobile_cache/src/repository/list_metadata.rs` - Repository for all three tables
- `wp_mobile_cache/src/db_types/list_metadata.rs` - DB types and column enums

Follow the patterns established in `posts.rs` and `term_relationships.rs`.

### Pagination State - DB as Source of Truth

When fetching the next page, the state transition function returns the page to fetch:

```rust
// MetadataService
pub fn begin_fetch_next_page(&self, key: &str) -> Result<Option<FetchNextPageInfo>, Error> {
    // In a transaction:
    // 1. Update state to FETCHING_NEXT_PAGE
    // 2. Read and return current_page + 1, version, etc.
    // Returns None if already at last page
}

pub struct FetchNextPageInfo {
    pub page: u32,
    pub version: u32,  // For concurrency check later
}
```

This approach:
- Forces state update before fetch (correct order)
- DB is single source of truth (no caching mismatch)
- No extra round trip (state update + read combined in one transaction)

### Entity State - Out of Scope

`EntityStateStore` remains in-memory. It's transient fetch state per entity, not list structure. May revisit in future but out of scope for this work.

### Key Generation - Future Centralization

With explicit `db_site_id` column, the key no longer embeds site_id. Format becomes `edit:posts:{status}`.

**Future improvement**: Centralize key generation in one place:

```rust
// Something like:
pub struct MetadataKey;

impl MetadataKey {
    pub fn post_list(filter: &AnyPostFilter) -> String {
        format!("edit:posts:{}", filter.status.as_ref().map(|s| s.to_string()).unwrap_or("all"))
    }

    pub fn comment_list(filter: &CommentFilter) -> String { ... }

    // All key generation in one place - easy to audit for uniqueness
}
```

This doesn't guarantee uniqueness programmatically, but centralizing makes collisions easy to spot and avoid. Can add tests to verify all generated keys are distinct.

**For now**: Key generation stays in PostService but follows the simplified format without site_id.
