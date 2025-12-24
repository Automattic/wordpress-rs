# MetadataCollection Composition Design

**Status: Implemented** ✅

## Goal

Refactor `MetadataCollection` to use composition, eliminating the `MetadataFetcher` trait and intermediate fetcher structs.

## Design

### Core (shared by all entity types)

```rust
struct MetadataCollectionCore {
    key: ListKey,
    metadata_reader: Arc<dyn ListMetadataReader>,
    state_reader: Arc<dyn EntityStateReader>,
    relevant_data_tables: Vec<DbTable>,
    per_page: u32,
}

impl MetadataCollectionCore {
    // Query methods - shared by all collections
    pub fn items(&self) -> Vec<CollectionItem> { ... }
    pub fn list_info(&self) -> Option<ListInfo> { ... }
    pub fn has_more_pages(&self) -> bool { ... }
    pub fn current_page(&self) -> u32 { ... }
    pub fn total_pages(&self) -> Option<u32> { ... }
    pub fn is_relevant_update(&self, hook: &UpdateHook) -> bool { ... }
}
```

### Entity-Specific Collections

Each entity type composes the core and adds its own fields:

```rust
struct PostMetadataCollectionWithEditContext {
    core: MetadataCollectionCore,
    service: Arc<PostService>,
    endpoint_type: PostEndpointType,
    filter: PostListFilter,
}

impl PostMetadataCollectionWithEditContext {
    pub async fn refresh(&self) -> Result<SyncResult, FetchError> {
        self.service.sync_list(
            self.core.key(),
            &self.endpoint_type,
            &self.filter,
            self.core.per_page(),
            true
        ).await
    }

    pub async fn load_next_page(&self) -> Result<SyncResult, FetchError> {
        // Early exit checks...
        self.service.sync_list(..., false).await
    }

    // Delegate to core
    pub fn items(&self) -> Vec<CollectionItem> { self.core.items() }
    pub fn list_info(&self) -> Option<ListInfo> { self.core.list_info() }
    // etc.
}
```

## Why Composition

- Core handles query infrastructure (items, pagination, relevance checks)
- Entity-specific collections own their filter and other entity-specific fields
- `endpoint_type` is Post-specific (Posts/Pages/Custom share this concept)
- Future entity types may need different fields (e.g., Comments might need `post_id`)
- Composition allows each entity-specific collection to extend the core naturally

## Changes Made

1. Renamed `MetadataCollection` → `MetadataCollectionCore`
2. Removed `MetadataFetcher` trait bound and generic parameter
3. Entity-specific collections store their own filter
4. Deleted `MetadataFetcher` trait
5. Deleted `PersistentPostMetadataFetcherWithEditContext`
