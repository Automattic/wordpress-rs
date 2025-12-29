# Metadata Collection Architecture

This document explains the architecture of the metadata-first collection system, including the reasoning behind key design decisions.

## Overview

The metadata collection system enables efficient list syncing by:
1. Fetching lightweight metadata (id + modified_gmt) to define list structure
2. Showing cached entities immediately with appropriate state indicators
3. Selectively fetching only entities that are missing or stale

## Architecture Diagram

See `metadata_orchestration_flow.png` for the visual representation.

## Layer Structure

### Client Layer (Kotlin/Swift)
- `PostMetadataCollectionViewModel` - Platform-specific view model
- `ObservableMetadataCollection` - Observable wrapper for reactive UI updates

### Collection Layer
- `PostMetadataCollectionWithEditContext` - Entity-specific collection that owns sync logic
- `MetadataCollectionCore` - Shared query infrastructure (joins metadata + state)

### Service Layer
- `PostService` - Entity-specific operations (fetch, sync, CRUD)
- `MetadataService` - Generic list metadata management (persistent)
- `EntityStateStore` - Per-entity fetch state tracking (in-memory)

### Repository Layer
- `ListMetadataRepository` - SQL operations for list metadata

### Database Tables
- `ListMetadata` - Pagination info (current_page, total_pages, total_items)
- `ListMetadataState` - Sync state (Idle, FetchingFirstPage, FetchingNextPage, Error)
- `ListMetadataItems` - List items (entity IDs with metadata)

## Key Design Decisions

### 1. MetadataCollectionCore as Composition

**What:** Entity-specific collections (e.g., `PostMetadataCollectionWithEditContext`) compose `MetadataCollectionCore` rather than inheriting from it.

**Why:**
- Core provides shared query logic that would otherwise be duplicated across entity types
- The `items()` method joins data from two sources (metadata reader + state reader)
- Relevance checking logic (`is_relevant_update`) contains non-trivial table filtering
- New entity types (Media, Comments, Users) can compose the core and add their own fields

**Structure:**
```rust
struct MetadataCollectionCore {
    key: ListKey,
    metadata_reader: Arc<dyn ListMetadataReader>,
    state_reader: Arc<dyn EntityStateReader>,
    relevant_data_tables: Vec<DbTable>,
    per_page: u32,
}

struct PostMetadataCollectionWithEditContext {
    core: MetadataCollectionCore,
    service: Arc<PostService>,
    endpoint_type: PostEndpointType,
    filter: PostListFilter,
}
```

### 2. Reader Traits for Interface Segregation

**What:** Collections receive `Arc<dyn ListMetadataReader>` and `Arc<dyn EntityStateReader>` instead of full service references.

**Why:**
- Limits what collections can do - they can only read, not write or delete
- Makes the code self-documenting - the type signature shows the contract
- Easier to reason about capabilities when reading the code
- Not primarily for testability (though it helps)

**Implementations:**
- `ListMetadataReader` is implemented by `MetadataService`
- `EntityStateReader` is implemented by `EntityStateStore`

### 3. Two Paths to MetadataService

**What:**
- Read path: `MetadataCollectionCore` → `MetadataService` (via `ListMetadataReader` trait)
- Write path: `PostMetadataCollectionWithEditContext` → `PostService` → `MetadataService`

**Why:**
- The read path uses narrow reader interfaces (interface segregation)
- The write path goes through `PostService` because sync involves both metadata AND fetching actual post entities
- `PostService` orchestrates the full sync flow: fetch metadata → detect stale → fetch missing entities
- This is semantically correct: when dealing with a post list, operations go through `PostService`

### 4. EntityStateStore in Service Layer (not Repository)

**What:** `EntityStateStore` lives in the Service Layer despite being a "store".

**Why:**
- It's in-memory only - resets on app restart
- Repository Layer is for persistent (SQL) storage
- Service Layer is for runtime state
- The layer distinction is about persistence, not naming

### 5. PostService Scope

**What:** `PostService` handles: network fetching, sync orchestration, DB operations, collection factories, state/reader providers.

**Why this is NOT a "god object":**
- The service is stateless (aside from `EntityStateStore` which is a simple cache)
- Methods are independent and composable
- No intertwined state that makes reasoning difficult
- All functionality is legitimately post-related
- Adding more methods doesn't increase complexity - they're orthogonal

**When to consider splitting:**
- Only if distinct responsibilities emerge that don't share dependencies
- Currently, all methods need access to `api_client`, `cache`, and `db_site`

### 6. Entity-Specific Collections Own Sync Logic

**What:** `PostMetadataCollectionWithEditContext.refresh()` and `load_next_page()` contain the sync implementation, not `MetadataCollectionCore`.

**Why:**
- Sync requires entity-specific knowledge (endpoint_type, filter)
- Sync calls `PostService.sync_list()` which handles post-specific concerns
- Core only handles query logic that's truly generic
- Future entity types may have different sync requirements

### 7. Filter Stored in Entity Collection, Not Core

**What:** `PostMetadataCollectionWithEditContext` stores its own `filter: PostListFilter`.

**Why:**
- Core doesn't use the filter - it just handles queries
- Some collections might not need a filter at all
- Keeps Core simple with no generic parameters
- Each entity collection stores exactly what it needs

## Data Flow

### Refresh Flow
1. `PostMetadataCollectionWithEditContext.refresh()` called
2. Calls `PostService.sync_list(key, endpoint_type, filter, per_page, is_refresh=true)`
3. `PostService` calls `MetadataService.refresh()` with a closure to fetch metadata
4. Metadata fetched via `PostService.fetch_posts_metadata()`
5. Metadata stored in database via `ListMetadataRepository`
6. Stale posts detected by comparing `modified_gmt`
7. Missing/stale posts loaded via `PostService.load_posts_by_ids()`
8. Entity states updated in `EntityStateStore`
9. Database update hooks fire, UI observers notified

### Query Flow
1. `PostMetadataCollectionWithEditContext.load_items()` called
2. Calls `core.items()` to get `CollectionItem` list (metadata + state)
3. Core reads from `ListMetadataReader` (→ `MetadataService` → DB)
4. Core reads from `EntityStateReader` (→ `EntityStateStore` → memory)
5. Collection loads full entity data from `PostService.read_posts_by_ids_from_db()`
6. Combines into `PostMetadataCollectionItem` with rich state enum

## Update Hook Flow

Database changes trigger `UpdateHook` notifications:
- `ListMetadata` / `ListMetadataState` / `ListMetadataItems` → list info/data changes
- `PostsEditContext` / `TermRelationships` → entity data changes

Collections check relevance via `is_relevant_update()` and notify observers.
