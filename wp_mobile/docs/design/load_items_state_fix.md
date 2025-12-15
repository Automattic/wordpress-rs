# Fix: `load_items()` Should Load Data Independent of EntityState

## Problem Statement

When reopening a posts page after initial load, posts display as "Post 12107" (ID-only
placeholder) instead of their actual titles, even though the data exists in the cache database.

### Observed Behavior

1. **First open**: User opens posts page, taps refresh
2. Posts load correctly with titles, excerpts, etc.
3. User navigates away or closes app
4. **Second open**: User opens posts page again
5. Posts show as "Post 12107", "Post 12105", etc. (placeholders)
6. User must tap refresh to see actual data

### Expected Behavior

On second open, posts should display their cached data immediately without requiring
a refresh.

---

## Root Cause Analysis

### The Bug: Conflating Fetch State with Cache Availability

The current implementation uses `EntityState` to determine whether to load data from
the cache. This conflates two independent concepts:

| Concept | Description |
|---------|-------------|
| **Fetch State** | Is a network fetch needed/in progress/completed/failed? |
| **Cache Availability** | Does data exist in the database for this entity? |

### Architecture Issue: Memory-Only State Store

From `entity_state_store.rs`:
```rust
/// Maps entity IDs to their current fetch state (Missing, Fetching, Cached, etc.).
/// This is a memory-only store - state resets on app restart.
pub struct EntityStateStore {
    states: RwLock<HashMap<i64, EntityState>>,
}
```

The `EntityStateStore` is a **memory-only HashMap**. When the app restarts:
- Metadata (post IDs) persists in the database
- Entity states reset to `Missing` (the default)

### Code Path Trace

```
App Restart → ViewModel.init() → createObservableCollection() → loadItemsFromCollection()
                                                                         ↓
PostMetadataCollectionWithEditContext::load_items()
                                                                         ↓
    let cached_ids: Vec<i64> = items
        .iter()
        .filter(|item| item.state.is_cached())  // ← State is Missing! Returns false
        .map(|item| item.id())
        .collect();  // cached_ids is empty!
                                                                         ↓
    // No posts loaded from DB because cached_ids is empty
    // Result: items have state=Missing, data=None
```

### In `post_metadata_collection.rs` (lines 110-153)

```rust
pub async fn load_items(&self) -> Result<Vec<PostMetadataCollectionItem>, CollectionError> {
    let items = self.collection.items();

    // BUG: Only loads data for items where state.is_cached() is true
    // After app restart, state resets to Missing, so nothing is loaded
    let cached_ids: Vec<i64> = items
        .iter()
        .filter(|item| item.state.is_cached())  // ← Problem is here
        .map(|item| item.id())
        .collect();

    let cached_posts = if cached_ids.is_empty() {
        Vec::new()
    } else {
        self.post_service.read_posts_by_ids_from_db(&cached_ids)?
    };
    // ...
}
```

---

## Proposed Solution

### Option A: Decouple Data Loading from Fetch State (Recommended)

**Principle**: Always attempt to load data from cache for ALL items, regardless of state.

The `EntityState` should only indicate whether a fetch is needed/in-progress, not
whether data might exist in the cache.

#### Implementation Changes

In `post_metadata_collection.rs`, modify `load_items()`:

```rust
pub async fn load_items(&self) -> Result<Vec<PostMetadataCollectionItem>, CollectionError> {
    let items = self.collection.items();

    // Load ALL items from cache - data availability is independent of fetch state
    let all_ids: Vec<i64> = items.iter().map(|item| item.id()).collect();

    let cached_posts = if all_ids.is_empty() {
        Vec::new()
    } else {
        self.post_service.read_posts_by_ids_from_db(&all_ids)?
    };

    // Build lookup map
    let mut cached_map: HashMap<i64, FullEntity<AnyPostWithEditContext>> =
        cached_posts.into_iter().map(|p| (p.data.id.0, p)).collect();

    // Combine items with their data (if available in cache)
    let result = items
        .into_iter()
        .map(|item| {
            // Data may exist regardless of state - try to get it
            let data = cached_map.remove(&item.id()).map(|e| e.into());

            PostMetadataCollectionItem {
                id: item.id(),
                state: item.state,
                data,
            }
        })
        .collect();

    Ok(result)
}
```

#### Pros
- Simple, minimal change
- No database schema changes
- Data availability is now independent of transient fetch state
- Works correctly on app restart

#### Cons
- Queries DB for all IDs every time (but batch query is efficient)
- May return data for items in `Failed` state (acceptable - shows last known data)

---

### Option B: Infer State from Cache

On collection creation, check which items have data in the cache and set their
initial state to `Cached` or `Stale` accordingly.

#### Implementation

In `MetadataCollection::new()` or a new initialization method:

```rust
pub fn initialize_states_from_cache(&self, post_service: &PostService) {
    let items = self.metadata_reader.get(&self.kv_key).unwrap_or_default();
    let ids: Vec<i64> = items.iter().map(|m| m.id).collect();

    // Check which IDs have data in cache
    let cached_ids = post_service.get_cached_post_ids(&ids);

    // Compare modified_gmt to determine Cached vs Stale
    for metadata in items {
        let state = if cached_ids.contains(&metadata.id) {
            if let Some(cached) = post_service.get_cached_modified_gmt(metadata.id) {
                if cached == metadata.modified_gmt {
                    EntityState::Cached
                } else {
                    EntityState::Stale
                }
            } else {
                EntityState::Missing
            }
        } else {
            EntityState::Missing
        };
        self.state_writer.set(metadata.id, state);
    }
}
```

#### Pros
- State accurately reflects cache status
- `is_cached()` continues to work as intended

#### Cons
- More complex implementation
- Requires additional DB queries on initialization
- Need to add `modified_gmt` storage to post cache table

---

### Option C: Persist EntityState to Database

Store `EntityState` in the database instead of memory.

#### Pros
- State persists across app restarts
- Most accurate representation

#### Cons
- Significant schema change
- More complex state management
- Need to handle state cleanup for deleted entities
- Overkill for this use case

---

## Recommendation

**Implement Option A** - Decouple data loading from fetch state.

This is the simplest fix that addresses the root cause: the `load_items()` function
should not use `EntityState` to decide whether data might exist in the cache.

### Key Insight

The `EntityState` enum represents the **fetch lifecycle**:
```
Missing → Fetching → Cached
              ↓
            Failed
```

It does NOT represent cache availability. Data can exist in the cache while state is:
- `Missing` (after app restart)
- `Stale` (modified_gmt mismatch, but old data still valid)
- `Failed` (fetch failed, but previous data may exist)

### Post-Fix Behavior

| State | Fetch Needed? | Load from Cache? | UI Shows |
|-------|--------------|------------------|----------|
| Missing | Yes | **Yes** (if exists) | Cached data or placeholder |
| Fetching | In progress | Yes (if exists) | Loading + cached data |
| Cached | No | Yes | Cached data |
| Stale | Yes | Yes | Cached data (may be outdated) |
| Failed | Retry? | Yes (if exists) | Cached data + error |

---

## Documentation Updates

Update the doc comment in `PostMetadataCollectionItem`:

```rust
/// Item in a metadata collection with optional loaded data.
///
/// Combines the collection item (id + state) with the full entity data
/// when available in the cache.
///
/// Note: `data` being `Some` is independent of `state`. Data may exist in
/// the cache while state is `Missing` (after app restart) or `Failed`
/// (showing last known data). Use `state` to determine fetch requirements,
/// not data availability.
#[derive(uniffi::Record)]
pub struct PostMetadataCollectionItem {
    /// The post ID
    pub id: i64,

    /// Current fetch state - indicates whether a fetch is needed/in-progress
    pub state: EntityState,

    /// Full entity data from cache, if available
    /// Note: May be present even when state is Missing, Stale, or Failed
    pub data: Option<crate::FullEntityAnyPostWithEditContext>,
}
```

---

## Testing

1. Load posts page, verify data loads correctly
2. Navigate away, return to posts page
3. Verify data shows immediately without refresh
4. Kill app, reopen, navigate to posts page
5. Verify data shows immediately (state will be Missing but data loads)
6. Tap refresh to verify sync still works correctly
