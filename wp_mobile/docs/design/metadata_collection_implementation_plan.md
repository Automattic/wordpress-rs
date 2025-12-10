# MetadataCollection Implementation Plan

This document tracks the implementation progress for the MetadataCollection design (v3).

## Branch: `prototype/metadata-collection`

## Design Document: `wp_mobile/docs/design/metadata_collection_v3.md`

---

## Order of Operations

### Phase 1: Core Types (no dependencies) ✅
- [x] **1.1** `EntityMetadata` - Struct with `i64` id + `Option<WpGmtDateTime>` (optional for entities without modified field)
- [x] **1.2** `EntityState` - Enum (Missing, Fetching, Cached, Stale, Failed)
- [x] **1.3** `CollectionItem` - Combines `EntityMetadata` + `EntityState`
- [x] **1.4** `SyncResult` & `MetadataFetchResult` - Result structs

**Commit:** `81a45b67` - "Add core types for MetadataCollection (v3 design)"

### Phase 2: Store Types ✅
- [x] **2.1** `EntityStateStore` + `EntityStateReader` trait
- [x] **2.2** `ListMetadataStore` + `ListMetadataReader` trait

**Commit:** `19f27529` - "Add EntityStateStore and ListMetadataStore"

### Phase 3: Collection Infrastructure ✅
- [x] **3.1** `MetadataFetcher` trait (async)
- [x] **3.2** `MetadataCollection<F>` struct

**Commit:** `aa9e4171` - "Add MetadataFetcher trait and MetadataCollection"

### Phase 4: Service Integration ✅
- [x] **4.1** Add stores as fields to `PostService`
- [x] **4.2** Add `fetch_and_store_metadata` method
- [x] **4.3** Update `fetch_posts_by_ids` to update state store
- [x] **4.4** Add `PostMetadataFetcherWithEditContext` concrete implementation
- [x] **4.5** Add reader accessor methods (`state_reader()`, `metadata_reader()`)
- [x] **4.6** Add `get_entity_state` helper method

**Commit:** `f295a6a5` - "Integrate MetadataCollection stores into PostService"

### Phase 5: Cleanup ✅
- [x] **5.1** Remove or refactor old sync module code that's superseded — N/A, no old code
- [x] **5.2** Update module exports — Already complete in Phase 4

**Note:** No cleanup needed - the sync module was built fresh with v3 design.

### Phase 6: UniFFI Export ✅
- [x] **6.1** Add `PostMetadataCollectionWithEditContext` concrete type
- [x] **6.2** Add `PostMetadataCollectionItem` record type (id + state + optional data)
- [x] **6.3** Add UniFFI derives to `EntityState` (Enum) and `SyncResult` (Record)
- [x] **6.4** Add interior mutability to `MetadataCollection` (`RwLock<PaginationState>`)
- [x] **6.5** Add `create_post_metadata_collection_with_edit_context` to PostService
- [x] **6.6** Add `read_posts_by_ids_from_db` helper method

**Commit:** `f735de18` - "Add PostMetadataCollectionWithEditContext for UniFFI export"

### Phase 7: Kotlin Wrapper (TODO)
- [ ] **7.1** Create `ObservableMetadataCollection` wrapper class
- [ ] **7.2** Register with `DatabaseChangeNotifier` for DB updates
- [ ] **7.3** Add extension function on `PostService` to create observable wrapper
- [ ] **7.4** Add TODO comment for state representation refinement

### Phase 8: Example App Screen (TODO)
- [ ] **8.1** Create `MetadataCollectionViewModel`
- [ ] **8.2** Create `MetadataCollectionScreen` composable
- [ ] **8.3** Wire up in navigation/DI

---

## Key Design Decisions (Quick Reference)

1. **No generics on stores** - IDs are `i64`, type safety at service boundary
2. **`Option<WpGmtDateTime>`** - Handles entities without `modified_gmt` (fallback to `last_fetched_at`)
3. **Service owns stores** - Collections get read-only access via traits
4. **Memory-only stores** - State resets on app restart
5. **Single fetch coordinator** - `fetch_posts_by_ids` is the funnel for state updates

---

## Current Progress

**Status:** Rust implementation complete, Kotlin wrapper next

**Last completed:** Phase 6 - UniFFI Export

**Next steps:** Phase 7 - Kotlin Wrapper (`ObservableMetadataCollection`)

---

## Notes

- `last_fetched_at` fallback for staleness check (for entities without `modified_gmt`) - implementation deferred
- State representation is simplified for prototype - see design doc "TODO: Refined State Representation" section
- DB observer fires before state store update (potential race) - acceptable for prototype
- `metadata_store` is shared across contexts (key includes context string), `state_store` is per-context
