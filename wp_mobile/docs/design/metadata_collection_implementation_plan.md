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

### Phase 4: Service Integration
- [ ] **4.1** Add stores as fields to `PostServiceWithEditContext`
- [ ] **4.2** Add `fetch_and_store_metadata` method
- [ ] **4.3** Update `fetch_posts_by_ids` to update state store
- [ ] **4.4** Add `PostMetadataFetcherWithEditContext` concrete implementation
- [ ] **4.5** Add reader accessor methods (`state_reader()`, `metadata_reader()`)

**Commit message:** "Integrate MetadataCollection into PostServiceWithEditContext"

### Phase 5: Cleanup
- [ ] **5.1** Remove or refactor old sync module code that's superseded
- [ ] **5.2** Update module exports

**Commit message:** "Clean up superseded MetadataCollection prototype code"

---

## Key Design Decisions (Quick Reference)

1. **No generics on stores** - IDs are `i64`, type safety at service boundary
2. **`Option<WpGmtDateTime>`** - Handles entities without `modified_gmt` (fallback to `last_fetched_at`)
3. **Service owns stores** - Collections get read-only access via traits
4. **Memory-only stores** - State resets on app restart
5. **Single fetch coordinator** - `fetch_posts_by_ids` is the funnel for state updates

---

## Current Progress

**Status:** Starting Phase 4

**Last completed:** Phase 3 - Collection Infrastructure (commit aa9e4171)

**Next task:** Integrate stores into `PostServiceWithEditContext`

---

## Notes

- Old prototype code exists in `wp_mobile/src/sync/` - will be superseded
- May need to add `DashMap` dependency for `EntityStateStore`
- `last_fetched_at` fallback for staleness check (for entities without `modified_gmt`) - implementation deferred
