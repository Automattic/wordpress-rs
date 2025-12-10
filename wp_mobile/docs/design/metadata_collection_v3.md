# MetadataCollection Design (v3) - Final

This document captures the finalized design for `MetadataCollection`, a generic collection type that uses a "metadata-first" sync strategy.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      PostServiceWithEditContext                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Owned Stores (memory-only):                                                 │
│  ┌─────────────────────────┐    ┌─────────────────────────────────────────┐ │
│  │    EntityStateStore     │    │         ListMetadataStore               │ │
│  │                         │    │                                         │ │
│  │  DashMap<i64,           │    │  RwLock<HashMap<                        │ │
│  │          EntityState>   │    │    String,              // filter key   │ │
│  │                         │    │    Vec<EntityMetadata>  // id + mod_gmt │ │
│  │  Per-entity fetch state │    │  >>                                     │ │
│  │  (Missing, Fetching,    │    │                                         │ │
│  │   Cached, Stale,        │    │  List structure per filter              │ │
│  │   Failed)               │    │  ("site_1:publish:date_desc" → [...])   │ │
│  └────────────┬────────────┘    └──────────────────┬──────────────────────┘ │
│               │                                     │                        │
│               │ writes                              │ writes                 │
│               │                                     │                        │
│  ┌────────────┴─────────────────────────────────────┴──────────────────────┐ │
│  │                                                                          │ │
│  │  fetch_posts_by_ids(ids: Vec<PostId>) → Result<(), FetchError>          │ │
│  │    1. Filter ids where state != Fetching                                │ │
│  │    2. Set filtered ids to Fetching in EntityStateStore                  │ │
│  │    3. Chunk into batches of 100 (API limit)                             │ │
│  │    4. Fetch each batch, upsert to DB                                    │ │
│  │    5. Set succeeded to Cached, failed to Failed                         │ │
│  │                                                                          │ │
│  │  fetch_and_store_metadata(kv_key, filter, page, per_page, is_first)     │ │
│  │    1. Fetch metadata from API (_fields=id,modified_gmt)                 │ │
│  │    2. If is_first: replace in ListMetadataStore                         │ │
│  │    3. Else: append in ListMetadataStore                                 │ │
│  │    4. Return MetadataFetchResult                                        │ │
│  │                                                                          │ │
│  │  get_entity_state(id: PostId) → EntityState                             │ │
│  │    → reads from EntityStateStore                                        │ │
│  │                                                                          │ │
│  └──────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  Read-only access (via traits):                                              │
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
│                                                                              │
│  kv_key: String                                                              │
│  metadata_reader: Arc<dyn ListMetadataReader>      // read-only              │
│  state_reader: Arc<dyn EntityStateReader>          // read-only              │
│  fetcher: F                                        // impl MetadataFetcher   │
│  relevant_tables: Vec<DbTable>                                               │
│                                                                              │
│  ───────────────────────────────────────────────────────────────────────     │
│                                                                              │
│  items() → Vec<CollectionItem>                                               │
│    → reads metadata from metadata_reader                                     │
│    → reads state for each from state_reader                                  │
│    → returns CollectionItem { metadata, state } for each                     │
│                                                                              │
│  refresh() → Result<SyncResult, FetchError>                                  │
│    → fetcher.fetch_metadata(page=1, is_first=true)                          │
│    → fetcher.ensure_fetched(missing_or_stale_ids)                           │
│                                                                              │
│  load_next_page() → Result<SyncResult, FetchError>                          │
│    → fetcher.fetch_metadata(next_page, is_first=false)                      │
│    → fetcher.ensure_fetched(missing_or_stale_ids)                           │
│                                                                              │
│  is_relevant_update(hook: &UpdateHook) → bool                               │
│    → relevant_tables.contains(&hook.table)                                  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
                               │
                               │ F: MetadataFetcher
                               ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                      MetadataFetcher (trait)                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  async fn fetch_metadata(&self, page: u32, per_page: u32, is_first: bool)   │
│      → Result<MetadataFetchResult, FetchError>                              │
│                                                                              │
│  async fn ensure_fetched(&self, ids: Vec<i64>)                              │
│      → Result<(), FetchError>                                                │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
                               │
                               │ Implemented by
                               ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│              PostMetadataFetcherWithEditContext (example)                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  service: &PostServiceWithEditContext                                        │
│  filter: AnyPostFilter                                                       │
│  kv_key: String                                                              │
│                                                                              │
│  fetch_metadata(page, per_page, is_first) → delegates to:                   │
│    → service.fetch_and_store_metadata(kv_key, filter, page, per_page, is_first)
│                                                                              │
│  ensure_fetched(ids) → delegates to:                                         │
│    → service.fetch_posts_by_ids(ids.map(PostId))                            │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Type Definitions

### EntityMetadata

Lightweight metadata for list structure. No generic - ID is raw `i64`.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityMetadata {
    pub id: i64,
    pub modified_gmt: WpGmtDateTime,
}
```

### EntityState

Fetch state for an entity. Tracked per-entity in the service's state store.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityState {
    /// Not in cache, not being fetched
    Missing,

    /// Fetch in progress
    Fetching,

    /// In cache and fresh (modified_gmt matches)
    Cached,

    /// In cache but outdated (modified_gmt mismatch)
    Stale,

    /// Fetch was attempted but failed
    Failed { error: String },
}
```

### CollectionItem

What the collection returns for each item. Combines metadata with current state.

```rust
#[derive(Debug, Clone)]
pub struct CollectionItem {
    pub metadata: EntityMetadata,
    pub state: EntityState,
}
```

Platform wraps each `CollectionItem` as observable. `loadData()` on the item loads the full entity from cache.

### SyncResult

Result of refresh/load_next_page operations.

```rust
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Number of items in the list after sync
    pub total_items: usize,

    /// Number of items that were fetched (missing + stale)
    pub fetched_count: usize,

    /// Number of items that failed to fetch
    pub failed_count: usize,

    /// Whether there are more pages available
    pub has_more_pages: bool,
}
```

### MetadataFetchResult

Result from metadata fetch (before full entity fetch).

```rust
#[derive(Debug, Clone)]
pub struct MetadataFetchResult {
    pub metadata: Vec<EntityMetadata>,
    pub total_items: Option<i64>,
    pub total_pages: Option<u32>,
    pub current_page: u32,
}
```

---

## Store Types

### EntityStateStore

Per-entity fetch state. Memory-only. Owned by service.

```rust
pub struct EntityStateStore {
    states: DashMap<i64, EntityState>,
}

impl EntityStateStore {
    pub fn get(&self, id: i64) -> EntityState {
        self.states.get(&id).map(|r| r.clone()).unwrap_or(EntityState::Missing)
    }

    pub fn set(&self, id: i64, state: EntityState) {
        self.states.insert(id, state);
    }

    pub fn set_batch(&self, ids: &[i64], state: EntityState) {
        ids.iter().for_each(|id| self.set(*id, state.clone()));
    }

    /// Get IDs that can be fetched (not currently Fetching)
    pub fn filter_fetchable(&self, ids: &[i64]) -> Vec<i64> {
        ids.iter()
            .filter(|id| !matches!(self.get(**id), EntityState::Fetching))
            .copied()
            .collect()
    }
}

// Read-only trait for collection
pub trait EntityStateReader: Send + Sync {
    fn get(&self, id: i64) -> EntityState;
}

impl EntityStateReader for EntityStateStore {
    fn get(&self, id: i64) -> EntityState {
        EntityStateStore::get(self, id)
    }
}
```

### ListMetadataStore

List structure per filter key. Memory-only (for now). Owned by service.

```rust
pub struct ListMetadataStore {
    data: RwLock<HashMap<String, Vec<EntityMetadata>>>,
}

impl ListMetadataStore {
    pub fn get(&self, key: &str) -> Option<Vec<EntityMetadata>> {
        self.data.read().unwrap().get(key).cloned()
    }

    pub fn set(&self, key: &str, metadata: Vec<EntityMetadata>) {
        self.data.write().unwrap().insert(key.to_string(), metadata);
    }

    pub fn append(&self, key: &str, metadata: Vec<EntityMetadata>) {
        self.data.write().unwrap()
            .entry(key.to_string())
            .or_default()
            .extend(metadata);
    }

    pub fn remove(&self, key: &str) {
        self.data.write().unwrap().remove(key);
    }
}

// Read-only trait for collection
pub trait ListMetadataReader: Send + Sync {
    fn get(&self, key: &str) -> Option<Vec<EntityMetadata>>;
}

impl ListMetadataReader for ListMetadataStore {
    fn get(&self, key: &str) -> Option<Vec<EntityMetadata>> {
        ListMetadataStore::get(self, key)
    }
}
```

---

## MetadataCollection

Generic collection type. No entity-specific logic.

```rust
pub struct MetadataCollection<F>
where
    F: MetadataFetcher,
{
    kv_key: String,
    metadata_reader: Arc<dyn ListMetadataReader>,
    state_reader: Arc<dyn EntityStateReader>,
    fetcher: F,
    relevant_tables: Vec<DbTable>,
    current_page: u32,
    total_pages: Option<u32>,
}

impl<F: MetadataFetcher> MetadataCollection<F> {
    pub fn new(
        kv_key: String,
        metadata_reader: Arc<dyn ListMetadataReader>,
        state_reader: Arc<dyn EntityStateReader>,
        fetcher: F,
        relevant_tables: Vec<DbTable>,
    ) -> Self {
        Self {
            kv_key,
            metadata_reader,
            state_reader,
            fetcher,
            relevant_tables,
            current_page: 0,
            total_pages: None,
        }
    }

    /// Get current items with their states
    pub fn items(&self) -> Vec<CollectionItem> {
        self.metadata_reader
            .get(&self.kv_key)
            .unwrap_or_default()
            .into_iter()
            .map(|metadata| CollectionItem {
                state: self.state_reader.get(metadata.id),
                metadata,
            })
            .collect()
    }

    /// Check if a DB update is relevant to this collection
    pub fn is_relevant_update(&self, hook: &UpdateHook) -> bool {
        self.relevant_tables.contains(&hook.table)
    }

    /// Refresh the collection (fetch page 1, replace metadata)
    pub async fn refresh(&mut self) -> Result<SyncResult, FetchError> {
        let result = self.fetcher.fetch_metadata(1, 20, true).await?;
        self.current_page = 1;
        self.total_pages = result.total_pages;

        self.sync_missing_and_stale().await
    }

    /// Load next page (append metadata)
    pub async fn load_next_page(&mut self) -> Result<SyncResult, FetchError> {
        let next_page = self.current_page + 1;

        if self.total_pages.map(|t| next_page > t).unwrap_or(false) {
            return Ok(SyncResult {
                total_items: self.items().len(),
                fetched_count: 0,
                failed_count: 0,
                has_more_pages: false,
            });
        }

        let result = self.fetcher.fetch_metadata(next_page, 20, false).await?;
        self.current_page = next_page;
        self.total_pages = result.total_pages;

        self.sync_missing_and_stale().await
    }

    /// Fetch missing and stale items
    async fn sync_missing_and_stale(&mut self) -> Result<SyncResult, FetchError> {
        let items = self.items();

        let ids_to_fetch: Vec<i64> = items
            .iter()
            .filter(|item| matches!(item.state, EntityState::Missing | EntityState::Stale | EntityState::Failed { .. }))
            .map(|item| item.metadata.id)
            .collect();

        let fetched_count = ids_to_fetch.len();

        if !ids_to_fetch.is_empty() {
            // Batch into chunks of 100 (API limit)
            for chunk in ids_to_fetch.chunks(100) {
                self.fetcher.ensure_fetched(chunk.to_vec()).await?;
            }
        }

        // Count failures after fetch attempt
        let failed_count = self.items()
            .iter()
            .filter(|item| matches!(item.state, EntityState::Failed { .. }))
            .count();

        Ok(SyncResult {
            total_items: items.len(),
            fetched_count,
            failed_count,
            has_more_pages: self.total_pages.map(|t| self.current_page < t).unwrap_or(true),
        })
    }

    /// Check if there are more pages to load
    pub fn has_more_pages(&self) -> bool {
        self.total_pages.map(|t| self.current_page < t).unwrap_or(true)
    }
}
```

---

## MetadataFetcher Trait

```rust
#[trait_variant::make(MetadataFetcher: Send)]
pub trait LocalMetadataFetcher {
    /// Fetch metadata for a page and store in ListMetadataStore
    ///
    /// If `is_first_page` is true, replaces existing metadata.
    /// Otherwise, appends to existing metadata.
    async fn fetch_metadata(
        &self,
        page: u32,
        per_page: u32,
        is_first_page: bool,
    ) -> Result<MetadataFetchResult, FetchError>;

    /// Ensure entities are fetched and cached
    ///
    /// Updates EntityStateStore appropriately (Fetching → Cached/Failed).
    async fn ensure_fetched(&self, ids: Vec<i64>) -> Result<(), FetchError>;
}
```

---

## Service Integration (PostServiceWithEditContext)

```rust
impl PostServiceWithEditContext {
    // Owned stores
    state_store: Arc<EntityStateStore>,
    metadata_store: Arc<ListMetadataStore>,

    /// Fetch metadata and store in ListMetadataStore
    pub async fn fetch_and_store_metadata(
        &self,
        kv_key: &str,
        filter: &AnyPostFilter,
        page: u32,
        per_page: u32,
        is_first_page: bool,
    ) -> Result<MetadataFetchResult, FetchError> {
        let params = /* build params from filter, page, per_page */;

        let response = self.api_client
            .posts()
            .filter_list_with_edit_context(
                &PostEndpointType::Posts,
                &params,
                &[SparseAnyPostFieldWithEditContext::Id,
                  SparseAnyPostFieldWithEditContext::ModifiedGmt],
            )
            .await?;

        let metadata: Vec<EntityMetadata> = response.data
            .iter()
            .filter_map(|sparse| Some(EntityMetadata {
                id: sparse.id?.0,  // unwrap PostId to i64
                modified_gmt: sparse.modified_gmt.clone()?,
            }))
            .collect();

        // Update store
        if is_first_page {
            self.metadata_store.set(kv_key, metadata.clone());
        } else {
            self.metadata_store.append(kv_key, metadata.clone());
        }

        Ok(MetadataFetchResult {
            metadata,
            total_items: response.header_map.wp_total(),
            total_pages: response.header_map.wp_total_pages(),
            current_page: page,
        })
    }

    /// Fetch posts by IDs, update state store, upsert to DB
    pub async fn fetch_posts_by_ids(&self, ids: Vec<PostId>) -> Result<(), FetchError> {
        let raw_ids: Vec<i64> = ids.iter().map(|id| id.0).collect();

        // Filter out already-fetching
        let fetchable = self.state_store.filter_fetchable(&raw_ids);
        if fetchable.is_empty() {
            return Ok(());
        }

        // Mark as fetching
        self.state_store.set_batch(&fetchable, EntityState::Fetching);

        // Fetch
        let post_ids: Vec<PostId> = fetchable.iter().map(|id| PostId(*id)).collect();
        let params = PostListParams {
            include: post_ids,
            ..Default::default()
        };

        match self.api_client.posts().list_with_edit_context(&PostEndpointType::Posts, &params).await {
            Ok(response) => {
                // Upsert to DB
                self.cache.execute(|conn| {
                    let repo = PostRepository::<EditContext>::new();
                    response.data.iter().try_for_each(|post| repo.upsert(conn, &self.db_site, post))
                })?;

                // Mark as cached
                let fetched_ids: Vec<i64> = response.data
                    .iter()
                    .filter_map(|p| p.id.map(|id| id.0))
                    .collect();
                self.state_store.set_batch(&fetched_ids, EntityState::Cached);

                // Mark missing as failed (requested but not returned)
                let failed_ids: Vec<i64> = fetchable
                    .iter()
                    .filter(|id| !fetched_ids.contains(id))
                    .copied()
                    .collect();
                self.state_store.set_batch(&failed_ids, EntityState::Failed {
                    error: "Not found".to_string(),
                });

                Ok(())
            }
            Err(e) => {
                // Mark all as failed
                self.state_store.set_batch(&fetchable, EntityState::Failed {
                    error: e.to_string(),
                });
                Err(e)
            }
        }
    }

    /// Get read-only access to stores (for MetadataCollection)
    pub fn state_reader(&self) -> Arc<dyn EntityStateReader> {
        self.state_store.clone()
    }

    pub fn metadata_reader(&self) -> Arc<dyn ListMetadataReader> {
        self.metadata_store.clone()
    }
}
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
                                         │
                                         │ retry
                                         ▼
                                    ┌──────────┐
                                    │ Fetching │
                                    └──────────┘
```

| Transition | Trigger |
|------------|---------|
| Missing → Fetching | `fetch_posts_by_ids` called |
| Fetching → Cached | Fetch succeeded, entity in DB |
| Fetching → Failed | Fetch failed or entity not returned |
| Cached → Stale | New metadata shows different `modified_gmt` |
| Stale → Fetching | `sync_missing_and_stale` or manual refresh |
| Failed → Fetching | Retry via `sync_missing_and_stale` |

---

## Cross-Collection Consistency

Because `EntityStateStore` lives in the service (not the collection):

- **Collection A** (All Posts) and **Collection B** (Published Posts) share the same state store
- Post 123 shows `Fetching` in both collections simultaneously
- Only one fetch request is made (service filters out already-fetching IDs)
- When fetch completes, both collections see `Cached` state

```
┌─────────────────────────────┐
│  PostServiceWithEditContext │
│  ┌───────────────────────┐  │
│  │   EntityStateStore    │  │
│  │   Post 123: Fetching  │◀─┼──── shared state
│  └───────────────────────┘  │
└─────────────────────────────┘
         ▲              ▲
         │              │
    ┌────┴────┐    ┌────┴────┐
    │ Coll A  │    │ Coll B  │
    │ (All)   │    │ (Pub)   │
    └─────────┘    └─────────┘
    Both see Post 123 as Fetching
```

---

## Summary

| Component | Generic? | Owns | Reads |
|-----------|----------|------|-------|
| `EntityStateStore` | No | Fetch state per entity (i64 key) | - |
| `ListMetadataStore` | No | List structure per filter (String key) | - |
| `PostServiceWithEditContext` | No | Both stores | - |
| `MetadataCollection<F>` | Yes (over F) | Nothing | Both stores via read-only traits |
| `MetadataFetcher` | No (trait) | Nothing | Delegates to service |
| `PostMetadataFetcher...` | No | Filter config | Service reference |

Key design principles:
1. **Service is the single coordinator** - all fetch logic, state updates, DB writes
2. **Collection is read-only** - just builds items from store data
3. **Stores use raw i64 for IDs** - type safety at service API boundary
4. **Memory-only stores** - simple, state resets on app restart
5. **Cross-collection consistency** - shared state store per service
