# MetadataCollection Design Document

## Overview

This document captures the design discussion for a new **generic** collection type that uses a "smart sync" strategy to efficiently fetch and display lists of entities. The key insight is to use lightweight metadata fetches to determine list structure, then selectively fetch only missing or stale full entity data.

**Important**: This design is not post-specific. Any WordPress REST API entity that has `id` and `modified_gmt` fields can use this pattern. This includes:
- Posts (and Pages, custom post types)
- Media
- Post Revisions
- Navigation Revisions
- Nav Menu Item Revisions
- Navigations

## Problem Statement

The current `PostCollection` always fetches full post data for every item in a list. This is inefficient when:
- Most posts are already cached and up-to-date
- The user only needs to see a list (not full content)
- Network bandwidth is limited

## Proposed Solution: MetadataCollection

A new collection type that:
1. Fetches lightweight metadata (id + modified_gmt) to define list structure
2. Shows cached posts immediately, with loading placeholders for missing items
3. Selectively fetches only posts that are missing or stale
4. Uses a KV store to persist list metadata across sessions

---

## Key Design Decisions

### 1. Metadata Defines List Structure (Not Just Sync Targets)

**User's insight**: The metadata fetch result should **define the list structure**, not just determine what needs syncing.

This means:
- The list order comes directly from the metadata fetch result
- If a post is in metadata but not in cache, show a **loading placeholder** at that position
- UI shows: `[Post 1] [Loading 2] [Post 3]` while Post 2 is being fetched

This provides better UX than:
- Showing only cached posts (incomplete list)
- Waiting for all data before showing anything (slow)

### 2. WordPress REST API Supports Batch Fetching

Confirmed that the `include` parameter supports fetching multiple posts by ID in a single request:

```
GET /wp/v2/posts?include=5,12,47&context=edit
```

This is already implemented in `PostListParams`:
```rust
pub struct PostListParams {
    // ...
    #[uniffi(default = [])]
    pub include: Vec<PostId>,
    // ...
}
```

This makes the selective fetch phase efficient - one request instead of N requests.

### 3. Memory vs DB-Backed Metadata Storage

**Discussion**: Should the list metadata (id + modified_gmt) be stored in the DB or kept in memory?

**Options considered**:

**Option A: DB-Backed (new table for metadata)**
- Pros: Instant UI on return, consistent ordering, offline resilience
- Cons: Schema complexity, migration overhead, cache invalidation complexity

**Option B: Memory-Based**
- Pros: Simpler implementation, no schema changes
- Cons: Cold start delay, no persistence across navigation

**Option C: KV Store (chosen approach)**
- Pros:
  - Persistence without schema changes to posts table
  - Clean separation: posts table = full data, KV store = list structure
  - Can start in-memory, easily switch to disk-based later
  - Page-aware (can store per-page or concatenated)
  - Filter-specific (different filters get their own entries)
  - Easy invalidation (clear key on refresh)
- Cons: Additional abstraction layer

**Decision**: Use KV store approach. Decouples list metadata from posts table entirely.

### 4. Fallback Strategy

**User's clarification**: The posts table query is a **fallback for initial load**, not the primary mechanism.

Flow:
1. Check KV store for cached metadata → if exists, use it to build list
2. If KV store empty → optionally fall back to posts table query while metadata loads
3. Once metadata fetch completes → KV store becomes source of truth for list structure

### 5. New Collection Type vs Extending StatelessCollection

**Discussion**: Should we extend `StatelessCollection` or create a new type?

**Problem with extending**: `StatelessCollection` monitors `DbTable` for updates via `UpdateHook`. KV store changes don't trigger these hooks.

**Decision**: Create a new `MetadataCollection` type (Option A) that's purpose-built for this pattern.

Reasons:
- Explicit about what it is
- Can have its own update/notification mechanism for KV changes
- Cleaner separation of concerns
- More flexibility for future evolution

---

## Detailed Flow

### Initial Load (Immediate)

```
1. kv_store.get(filter_key) → Option<Vec<PostMetadata>>

2. If Some(metadata):
   - For each item in metadata:
     - Query cache for post by ID
     - If found AND fresh → include full post
     - If found BUT stale → include full post, mark for refresh
     - If not found → include loading placeholder
   - Show list immediately

3. If None:
   - Option A: Show full loading state
   - Option B: Fallback to posts table query (SELECT * FROM posts WHERE status = 'publish' ORDER BY date DESC)
```

### Background Sync

```
1. Metadata Fetch:
   GET /wp/v2/posts?status=publish&_fields=id,modified_gmt&orderby=date&order=desc&page=1

   Returns: [{id: 1, modified_gmt: "2024-01-15T10:00:00"}, {id: 2, modified_gmt: "2024-01-14T09:00:00"}, ...]

2. Update KV Store:
   - Page 1: kv_store.set(filter_key, metadata)  // replace
   - Page N: kv_store.append(filter_key, metadata)  // append

3. Diff with Cache:
   For each PostMetadata in result:
     - Check if post exists in posts table
     - Compare modified_gmt with cached value
     - Build list of missing or stale post IDs

4. Batch Fetch Missing/Stale:
   GET /wp/v2/posts?include=2,5,8&context=edit

   → Upsert full post data to posts table
   → DB UpdateHook triggers
   → UI re-renders affected items
```

### Pagination ("Load More")

```
User scrolls to bottom → fetch_page(2)

1. GET /wp/v2/posts?status=publish&_fields=id,modified_gmt&page=2
2. kv_store.append(filter_key, page_2_metadata)
3. Diff and batch fetch missing/stale
4. UI appends new items
```

---

## API Design Sketch

### Generic Traits and Types

```rust
/// Trait for entities that support metadata-based sync
///
/// Any WordPress REST API entity with `id` and `modified_gmt` fields
/// can implement this trait to work with MetadataCollection.
pub trait SyncableEntity {
    /// The ID type for this entity (e.g., PostId, MediaId)
    type Id: Clone + Eq + std::hash::Hash + Send + Sync;

    fn id(&self) -> Option<Self::Id>;
    fn modified_gmt(&self) -> Option<&WpGmtDateTime>;
}

// Example implementations:
impl SyncableEntity for SparseAnyPostWithEditContext {
    type Id = PostId;

    fn id(&self) -> Option<Self::Id> { self.id }
    fn modified_gmt(&self) -> Option<&WpGmtDateTime> { self.modified_gmt.as_ref() }
}

impl SyncableEntity for SparseMediaWithEditContext {
    type Id = MediaId;

    fn id(&self) -> Option<Self::Id> { self.id }
    fn modified_gmt(&self) -> Option<&WpGmtDateTime> { self.modified_gmt.as_ref() }
}
```

### EntityMetadata (generic)

```rust
/// Lightweight metadata for any entity, used for list structure
#[derive(Debug, Clone, uniffi::Record)]
pub struct EntityMetadata<Id> {
    pub id: Id,
    pub modified_gmt: WpGmtDateTime,
}

// Type aliases for convenience
pub type PostMetadata = EntityMetadata<PostId>;
pub type MediaMetadata = EntityMetadata<MediaId>;
```

### MetadataCollection (generic)

```rust
/// Collection that uses metadata-first fetching strategy
///
/// Generic over:
/// - `T`: The full entity type (e.g., AnyPostWithEditContext)
/// - `Id`: The ID type (e.g., PostId)
pub struct MetadataCollection<T, Id>
where
    Id: Clone + Eq + std::hash::Hash + Send + Sync,
{
    /// Key for KV store lookup
    kv_key: String,

    /// KV store for metadata persistence
    kv_store: Arc<dyn KvStore<Id>>,

    /// Closure to fetch metadata from network
    fetch_metadata: Box<dyn Fn(u32, u32) -> Future<Output = Result<MetadataFetchResult<Id>, FetchError>>>,

    /// Closure to fetch full entities by IDs
    fetch_by_ids: Box<dyn Fn(Vec<Id>) -> Future<Output = Result<Vec<T>, FetchError>>>,

    /// Closure to load entities from cache given metadata
    load_from_cache: Box<dyn Fn(&[EntityMetadata<Id>]) -> Result<Vec<ListItem<T, Id>>, ...>>,
}
```

### ListItem (generic, either loaded or placeholder)

```rust
/// An item in an entity list - either fully loaded or a placeholder
#[derive(Debug, Clone, uniffi::Enum)]
pub enum ListItem<T, Id> {
    /// Fully loaded entity from cache
    Loaded(FullEntity<T>),

    /// Placeholder for entity being fetched
    Loading { id: Id },
}

// Type aliases for convenience
pub type PostListItem = ListItem<AnyPostWithEditContext, PostId>;
pub type MediaListItem = ListItem<MediaWithEditContext, MediaId>;
```

### KvStore Trait (generic)

```rust
/// Simple KV store abstraction - can be in-memory or persistent
///
/// Generic over the ID type to support different entity types.
pub trait KvStore<Id>: Send + Sync
where
    Id: Clone + Eq + std::hash::Hash,
{
    fn get(&self, key: &str) -> Option<Vec<EntityMetadata<Id>>>;
    fn set(&self, key: &str, value: Vec<EntityMetadata<Id>>);
    fn append(&self, key: &str, value: Vec<EntityMetadata<Id>>);
    fn remove(&self, key: &str);
}

/// Concrete in-memory implementation
pub struct InMemoryKvStore<Id> {
    data: RwLock<HashMap<String, Vec<EntityMetadata<Id>>>>,
}
```

### MetadataFetchResult (generic)

```rust
/// Result of a metadata fetch operation
#[derive(Debug, Clone)]
pub struct MetadataFetchResult<Id> {
    /// Metadata for entities in this page
    pub metadata: Vec<EntityMetadata<Id>>,

    /// Total number of items matching the query (from API)
    pub total_items: Option<i64>,

    /// Total number of pages available (from API)
    pub total_pages: Option<u32>,

    /// The page number that was fetched
    pub current_page: u32,
}
```

### Service Layer Addition (example for Posts)

```rust
impl PostService {
    /// Fetch only metadata (id + modified_gmt) for a page of posts
    pub async fn fetch_posts_metadata(
        &self,
        filter: &AnyPostFilter,
        page: u32,
        per_page: u32,
    ) -> Result<MetadataFetchResult<PostId>, FetchError> {
        let mut params = filter.to_list_params();
        params.page = Some(page);
        params.per_page = Some(per_page);

        let response = self
            .api_client
            .posts()
            .filter_list_with_edit_context(
                &PostEndpointType::Posts,
                &params,
                &[
                    SparseAnyPostFieldWithEditContext::Id,
                    SparseAnyPostFieldWithEditContext::ModifiedGmt,
                ],
            )
            .await?;

        // Map sparse posts to EntityMetadata
        let metadata: Vec<EntityMetadata<PostId>> = response
            .data
            .iter()
            .filter_map(|sparse| {
                Some(EntityMetadata {
                    id: sparse.id?,
                    modified_gmt: sparse.modified_gmt.clone()?,
                })
            })
            .collect();

        Ok(MetadataFetchResult {
            metadata,
            total_items: response.header_map.wp_total().map(|n| n as i64),
            total_pages: response.header_map.wp_total_pages(),
            current_page: page,
        })
    }

    /// Fetch full posts by their IDs (for selective sync)
    pub async fn fetch_posts_by_ids(
        &self,
        ids: Vec<PostId>,
    ) -> Result<Vec<AnyPostWithEditContext>, FetchError> {
        let params = PostListParams {
            include: ids,
            ..Default::default()
        };

        let response = self
            .api_client
            .posts()
            .list_with_edit_context(&PostEndpointType::Posts, &params)
            .await?;

        // Upsert to cache
        self.cache.execute(|conn| {
            let repo = PostRepository::<EditContext>::new();
            for post in &response.data {
                repo.upsert(conn, &self.db_site, post)?;
            }
            Ok(())
        })?;

        Ok(response.data)
    }
}

// Similar methods would be added to MediaService, etc.
```

---

## Open Questions / Future Refinements

### 1. KV Store Key Design

How to generate the key for KV store lookups:

- **Option A**: Hash of entire `AnyPostFilter` struct
- **Option B**: Simple string like `"{status}_{orderby}_{order}"`
- **Option C**: User-defined key passed when creating collection

Not a blocker - can start simple and refine.

### 2. Staleness Threshold

How to determine if a cached post is "stale":

- **Option A**: Compare `modified_gmt` only - if different, refetch
- **Option B**: Time-based - if cached more than X minutes ago, refetch
- **Option C**: Combination

Can be configurable, easy to change later.

### 3. KV Store Implementation

Initial implementation can be in-memory (`HashMap`), with easy swap to:
- SQLite-backed KV table
- File-based (serde to JSON/bincode)
- Platform-specific (UserDefaults on iOS, SharedPreferences on Android)

### 4. Update Notifications

How does the UI know when to re-render?

- DB changes to posts table → existing `UpdateHook` mechanism
- KV store metadata changes → new notification mechanism needed?

May need a callback/observer pattern for KV store changes, or the collection can expose a signal when metadata is updated.

### 5. Error Handling

What happens when:
- Metadata fetch fails → show cached list (from KV store or posts table fallback)
- Batch fetch fails for some posts → show cached version or error state per-item
- KV store read/write fails → fall back to memory-only mode

---

## Relationship to Existing Types

```
StatelessCollection<T>
├── Monitors DbTable for changes
├── load_data() queries DB directly
└── No network awareness

PostCollection<T>
├── Wraps StatelessCollection
├── Adds filter configuration
├── fetch_page() does full post fetch + upsert
└── load_data() delegates to StatelessCollection

MetadataCollection<T> (NEW)
├── Uses KV store for list structure
├── fetch_metadata() does lightweight fetch
├── fetch_missing() does selective batch fetch
├── load_data() returns PostListItem<T> (loaded or placeholder)
└── Separate from StatelessCollection (different pattern)
```

---

## Summary

The `MetadataCollection` provides an efficient sync strategy:

1. **Lightweight metadata defines list structure** - fast, shows order/count immediately
2. **Loading placeholders for missing items** - great UX, user sees list skeleton
3. **Selective batch fetch** - only fetch what's needed, single request via `include` param
4. **KV store for persistence** - survives navigation, easy to swap implementations
5. **Clean separation** - posts table holds full data, KV store holds list structure

This approach optimizes for the common case where most posts are cached and up-to-date, while still handling new/updated posts gracefully.

---

## Implementation Status

**Branch**: `prototype/metadata-collection`

### Completed Components

| Component | Location | Description |
|-----------|----------|-------------|
| `SyncableEntity` trait | `wp_mobile/src/sync/syncable_entity.rs` | Trait for entities with `id` + `modified_gmt` |
| `EntityMetadata<Id>` | `wp_mobile/src/sync/entity_metadata.rs` | Lightweight metadata struct |
| `ListItem<T, Id>` | `wp_mobile/src/sync/list_item.rs` | Enum with `Loaded`, `Loading`, `Failed` variants |
| `KvStore<Id>` trait | `wp_mobile/src/sync/kv_store.rs` | Abstraction for metadata persistence |
| `InMemoryKvStore<Id>` | `wp_mobile/src/sync/kv_store.rs` | In-memory implementation |
| `MetadataFetchResult<Id>` | `wp_mobile/src/sync/metadata_fetch_result.rs` | Result type for metadata fetches |
| `MetadataCollection<T, Id>` | `wp_mobile/src/sync/metadata_collection.rs` | Core collection type |
| `fetch_posts_metadata()` | `wp_mobile/src/service/posts.rs` | Lightweight metadata fetch |
| `fetch_posts_by_ids()` | `wp_mobile/src/service/posts.rs` | Batch fetch by IDs |

### Supporting Changes

- Added `Clone`, `Copy` to `WpGmtDateTime` (`wp_api/src/date.rs`)
- Added `Hash` to `wp_content_i64_id!` and `wp_content_u64_id!` macros (`wp_api/src/wp_content_macros.rs`)

### Test Coverage

- 6 tests for `InMemoryKvStore`
- 9 tests for `MetadataCollection`
- All 24 `wp_mobile` lib tests passing

### Differences from Original Sketch

1. **`MetadataCollection` uses closures instead of storing fetch functions**
   - Original: Had `fetch_metadata` and `fetch_by_ids` closures in the struct
   - Implemented: Uses `load_entity_by_id` and `get_cached_modified_gmt` closures; fetching is done externally via `PostService`

2. **`ListItem` has three states, not two**
   - Original: `Loaded` and `Loading` only
   - Implemented: Added `Failed { metadata, error }` for error handling

3. **`ListItem::Loading` holds full metadata, not just ID**
   - Original: `Loading { id: Id }`
   - Implemented: `Loading(EntityMetadata<Id>)` - preserves `modified_gmt` for display

4. **Type aliases for closure types**
   - Added `EntityLoader<T, Id>` and `ModifiedGmtLoader<Id>` to satisfy clippy's type complexity warnings

### Next Steps

1. Create a concrete `PostMetadataCollection` wrapper (similar to `PostCollectionWithEditContext`)
2. Add method to get `modified_gmt` from cached posts in repository layer
3. Integrate with platform-specific observable wrappers (iOS/Android)
4. Consider disk-backed `KvStore` implementation

---

## Revised Design (v2) - Fully Generic Collection

This revision moves toward a fully generic collection that doesn't need type-specific wrappers (except for uniffi).

### Key Insights

The collection follows the same pattern as `StatelessCollection`:
- Rust side is a **handle** with `is_relevant_update()` and data accessors
- Platform layer (Kotlin/Swift) wraps it as observable
- `loadData()` is called by observers, not returned by collection methods

Each **list item** becomes individually observable (like `ObservableEntity`):
- Item holds `EntityMetadata<Id>` (id + modified_gmt)
- Platform layer wraps each item as observable
- `loadData()` on the item loads that specific entity from cache

### Proposed Types

#### MetadataCollection (Rust handle)

```rust
pub struct MetadataCollection<Id, F>
where
    Id: Clone + Eq + Hash + Send + Sync,
    F: MetadataFetcher<Id>,
{
    kv_key: String,
    kv_store: Arc<dyn KvStore<Id>>,
    metadata: Option<Vec<EntityMetadata<Id>>>,
    fetcher: F,
    relevant_tables: Vec<DbTable>,
}

impl<Id, F> MetadataCollection<Id, F> {
    /// Get current metadata items (for platform to wrap as observable)
    pub fn items(&self) -> Option<&[EntityMetadata<Id>]> {
        self.metadata.as_deref()
    }

    /// Check if update is relevant to this collection
    pub fn is_relevant_update(&self, hook: &UpdateHook) -> bool {
        self.relevant_tables.contains(&hook.table)
    }

    /// Refresh metadata from network
    pub async fn refresh(&mut self) -> Result<(), FetchError>;

    /// Load next page
    pub async fn load_next_page(&mut self) -> Result<(), FetchError>;
}
```

#### MetadataFetcher Trait (no T parameter)

```rust
pub trait MetadataFetcher<Id> {
    async fn fetch_metadata(&self, page: u32, per_page: u32)
        -> Result<MetadataFetchResult<Id>, FetchError>;

    // Fetches by IDs and puts in cache - no return needed
    async fn fetch_by_ids(&self, ids: Vec<Id>)
        -> Result<(), FetchError>;
}
```

### Open Questions

1. **KV Store Update Responsibility**: The `MetadataFetcher` implementation (e.g., for posts) needs to update the KV store. Service layer orchestrates whether to replace (first page) or append (subsequent pages).

2. **Who calls `fetch_by_ids`?**: Service layer is natural fit, but issues:
   - What happens when the request fails?
   - What if missing >100 posts (API limit)?

3. **Error Handling for Batch Fetches**: TBD

4. **Batching Strategy for Large Missing Sets**: TBD
