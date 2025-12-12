# MetadataCollection Design & Implementation

A "metadata-first" sync strategy for efficient list fetching in WordPress mobile apps.

## Overview

MetadataCollection uses lightweight metadata (id + modified_gmt) to define list structure, then selectively fetches only missing or stale entities. This optimizes for the common case where most posts are cached.

**Key features:**
- Database-backed list metadata with pagination persistence
- Split observers for data vs state updates
- State persistence across filter changes and app restarts
- Cross-collection consistency (shared entity state)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              PostService                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────┐    ┌─────────────────────────────────────────┐ │
│  │ state_store_with_       │    │         MetadataService                 │ │
│  │      edit_context       │    │                                         │ │
│  │                         │    │  DB Tables:                             │ │
│  │  Memory-only HashMap    │    │  - list_metadata (pagination)           │ │
│  │  i64 → EntityState      │    │  - list_metadata_items (entity IDs)     │ │
│  │                         │    │  - list_metadata_state (sync state)     │ │
│  │  Per-entity fetch state │    │                                         │ │
│  │  (Missing, Fetching,    │    │  Persists across app restarts           │ │
│  │   Cached, Stale,        │    │                                         │ │
│  │   Failed)               │    │                                         │ │
│  └────────────┬────────────┘    └──────────────────┬──────────────────────┘ │
│               │ writes                              │ writes                 │
│               │                                     │                        │
│  ┌────────────┴─────────────────────────────────────┴──────────────────────┐ │
│  │  fetch_and_store_metadata_persistent(key, filter, page, per_page)       │ │
│  │    1. begin_refresh() or begin_fetch_next_page()                        │ │
│  │    2. Fetch metadata from API (_fields=id,modified_gmt)                 │ │
│  │    3. Store items via MetadataService                                   │ │
│  │    4. Detect staleness (compare modified_gmt with cached)               │ │
│  │    5. complete_sync() or complete_sync_with_error()                     │ │
│  │                                                                          │ │
│  │  fetch_posts_by_ids(ids) → fetches missing/stale, updates state_store   │ │
│  └──────────────────────────────────────────────────────────────────────────┘ │
│               │                                     │                        │
│               │ Arc<dyn EntityStateReader>          │ Arc<dyn ListMetadataReader>
│               ▼                                     ▼                        │
└─────────────────────────────────────────────────────────────────────────────┘
                │                                     │
                └──────────────┬──────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         MetadataCollection<F>                                │
├─────────────────────────────────────────────────────────────────────────────┤
│  kv_key: String                                                              │
│  metadata_reader: Arc<dyn ListMetadataReader>      // read-only              │
│  state_reader: Arc<dyn EntityStateReader>          // read-only              │
│  fetcher: F                                        // impl MetadataFetcher   │
│                                                                              │
│  items() → Vec<CollectionItem>        // reads from DB                       │
│  refresh() → SyncResult               // fetch page 1, sync missing/stale    │
│  load_next_page() → SyncResult        // fetch next page, sync               │
│  is_relevant_data_update(hook) → bool // entity tables + ListMetadataItems   │
│  is_relevant_state_update(hook) → bool// ListMetadataState table             │
│  sync_state() → ListState             // Idle, FetchingFirstPage, etc.       │
└─────────────────────────────────────────────────────────────────────────────┘
                               │
                               │ Kotlin/Swift wrapper
                               ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    ObservableMetadataCollection                              │
├─────────────────────────────────────────────────────────────────────────────┤
│  dataObservers: List<() -> Unit>      // notified on list content changes    │
│  stateObservers: List<() -> Unit>     // notified on sync state changes      │
│                                                                              │
│  addDataObserver(observer)            // for list UI                         │
│  addStateObserver(observer)           // for loading indicators              │
│  notifyIfRelevant(hook)               // routes DB updates to observers      │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Status

| Phase | Description | Status | Commit |
|-------|-------------|--------|--------|
| 1.1-1.5 | Database tables, types, repository | ✅ | `3c95dfb4` |
| 1.6 | Concurrency helpers (begin_refresh, etc.) | ✅ | `e484f791` |
| 2.1-2.4 | MetadataService wrapper | ✅ | `3c85514b` |
| 3.1 | Collection closure pattern | ⏸️ Deferred | - |
| 3.2 | PostService integration | ✅ | `5c83b435` |
| 3.3 | Persistent fetcher | ✅ | `7854e9e7` |
| 3.4 | Remove in-memory store | ✅ | `95a2db5f` |
| 4.1 | Split is_relevant_update | ✅ | `ef4d65d0` |
| 4.2 | Kotlin split observers | ✅ | `c29bcd50` |
| 4.3 | sync_state() method | ✅ | `ef4d65d0` |
| 5.1-5.2 | Repository & service tests | ✅ | (inline) |
| 5.3 | Example app UI | ✅ | `c29bcd50` |
| 5.4 | Bug fixes | ✅ | `30c69218` |
| 5.5 | Debug print cleanup | ✅ | `0b120639` |
| 5.6 | State persistence | ✅ | `30c69218` |

---

## Key Files

| File | Purpose |
|------|---------|
| `wp_mobile_cache/migrations/0007-create-list-metadata-tables.sql` | Schema |
| `wp_mobile_cache/src/list_metadata.rs` | `ListState` enum, structs |
| `wp_mobile_cache/src/repository/list_metadata.rs` | Repository (31 tests) |
| `wp_mobile/src/service/metadata.rs` | MetadataService (15 tests) |
| `wp_mobile/src/service/posts.rs` | PostService integration |
| `wp_mobile/src/sync/metadata_collection.rs` | Generic collection |
| `wp_mobile/src/sync/list_metadata_reader.rs` | Read-only trait |
| `native/kotlin/.../ObservableMetadataCollection.kt` | Kotlin wrapper |

---

## Database Schema

```sql
-- List header/pagination
CREATE TABLE list_metadata (
  rowid INTEGER PRIMARY KEY,
  db_site_id INTEGER NOT NULL,
  key TEXT NOT NULL,              -- "edit:posts:publish"
  total_pages INTEGER,
  current_page INTEGER DEFAULT 0,
  per_page INTEGER DEFAULT 20,
  version INTEGER DEFAULT 0,      -- for concurrency control
  FOREIGN KEY (db_site_id) REFERENCES db_sites(id)
);

-- List items (rowid = display order)
CREATE TABLE list_metadata_items (
  rowid INTEGER PRIMARY KEY,
  db_site_id INTEGER NOT NULL,
  key TEXT NOT NULL,
  entity_id INTEGER NOT NULL,
  modified_gmt TEXT
);

-- Sync state (separate for efficient observers)
CREATE TABLE list_metadata_state (
  rowid INTEGER PRIMARY KEY,
  list_metadata_id INTEGER NOT NULL,
  state TEXT DEFAULT 'idle',      -- idle, fetching_first_page, fetching_next_page, error
  error_message TEXT,
  FOREIGN KEY (list_metadata_id) REFERENCES list_metadata(rowid)
);
```

---

## State Transitions

```
                       ┌─────────────────────────────────────┐
                       │                                     │
                       ▼                                     │
┌─────────┐       ┌──────────┐       ┌────────┐       ┌──────┴──┐
│ Missing │──────▶│ Fetching │──────▶│ Cached │──────▶│  Stale  │
└─────────┘       └──────────┘       └────────┘       └─────────┘
                       │                                   │
                       │             ┌────────┐            │
                       └────────────▶│ Failed │◀───────────┘
                                     └────────┘
```

| Transition | Trigger |
|------------|---------|
| Missing → Fetching | `fetch_posts_by_ids` called |
| Fetching → Cached | Fetch succeeded |
| Fetching → Failed | Fetch failed or entity not in response |
| Cached → Stale | New metadata has different `modified_gmt` |
| Stale/Failed → Fetching | Retry via refresh or load_next_page |

---

## Key Design Decisions

### 1. Service owns stores, collection reads only
Collections get `Arc<dyn ListMetadataReader>` - no direct write access. Single coordination point prevents race conditions.

### 2. Split data vs state observers
- **Data observers**: Fire when list content changes (posts table, list_metadata_items)
- **State observers**: Fire when sync state changes (list_metadata_state)

Allows efficient UI updates - loading spinners don't trigger full list reloads.

### 3. Async load_items() and sync_state()
SQLite update hooks fire synchronously during transactions. If hook callbacks query the DB, deadlock occurs. Making these async lets UniFFI dispatch to background threads.

### 4. Simplified relevance checks
`is_relevant_update()` only checks table names, not keys. False positives (extra refreshes) are acceptable; deadlocks are not.

### 5. Stale state reset on app launch
Transient states (`FetchingFirstPage`, `FetchingNextPage`) are reset to `Idle` in `WpApiCache::perform_migrations()`. Prevents stuck loading indicators after crashes.

### 6. Collection loads pagination from DB on creation
`MetadataCollection::new()` reads `current_page` and `total_pages` from database. State persists across filter changes and app restarts.

---

## Bug Fixes

### Race Condition in ViewModel State Updates
**Problem**: UI stuck on "Fetching Next Page" when logs showed IDLE.
**Cause**: Completion handlers did `_state.value.copy(isSyncing = false)` without `syncState`, overwriting observer updates.
**Fix**: Completion handlers now set `syncState = collection.syncState()`.

### State Not Persisting on Filter Change
**Problem**: `Page: 0` shown for previously-fetched filters.
**Cause**: `MetadataCollection::new()` hardcoded `current_page: 0`.
**Fix**: Added `get_current_page()` and `get_total_pages()` to `ListMetadataReader` trait.

### Deadlock in Hook Callbacks
**Problem**: App froze when DB updates triggered observers.
**Cause**: Synchronous hook callbacks tried to query DB held by transaction.
**Fix**: Made `load_items()` and `sync_state()` async; simplified relevance checks.

---

## Commit History

| Commit | Description |
|--------|-------------|
| `3c95dfb4` | Add database foundation for MetadataService (Phase 1) |
| `e484f791` | Add list metadata repository concurrency helpers |
| `3c85514b` | Add MetadataService for database-backed list metadata |
| `5c83b435` | Integrate MetadataService into PostService |
| `7854e9e7` | Update PostMetadataCollection to use database-backed storage |
| `ef4d65d0` | Split collection observers for data vs state updates |
| `c29bcd50` | Complete Phase 4 & 5: Split observers, async methods, UI |
| `95a2db5f` | Remove deprecated in-memory metadata store |
| `0b120639` | Clean up debug prints for better readability |
| `30c69218` | Fix state persistence when switching filters |
| `c80ae6a8` | Update documentation |
| `817e5f76` | Fix tests and detekt issues |

---

## Test Coverage

- `wp_mobile_cache`: 112 tests (31 for list_metadata repository)
- `wp_mobile`: 60 tests (15 for MetadataService)
