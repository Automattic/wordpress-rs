# Documentation Update Report

This report identifies outdated documentation that needs updating after the metadata collection refactoring.

## New Document Created

- **`metadata_collection_architecture.md`** - Comprehensive design rationale document explaining why everything is where it is. This should be the primary reference.

## Documents Requiring Updates

### 1. `metadata_collection.md` - MAJOR UPDATES NEEDED

**Outdated elements:**

| Section | Issue | Fix |
|---------|-------|-----|
| Architecture diagram | Shows `MetadataCollection<F>` with `fetcher: F` | Update to `MetadataCollectionCore` (no generic) |
| Architecture diagram | Shows `fetch_and_store_metadata_persistent` | Remove - this method was deleted |
| Architecture diagram | Shows `MetadataFetcher` trait | Remove - trait was deleted |
| Line 57-70 | `MetadataCollection<F>` struct definition | Update to show `MetadataCollectionCore` and `PostMetadataCollectionWithEditContext` |
| Line 62 | `fetcher: F // impl MetadataFetcher` | Remove - collections own sync logic now |
| Key Files table | `wp_mobile/src/sync/metadata_collection.rs` described as "Generic collection" | Update description |
| Key Files table | Missing `post_metadata_collection.rs` | Add entry for entity-specific collection |

**Recommendation:** Either significantly update this document or mark it as historical and point to `metadata_collection_architecture.md`.

### 2. `metadata_service_orchestration.md` - MODERATE UPDATES NEEDED

**Outdated elements:**

| Section | Issue | Fix |
|---------|-------|-----|
| Line 79-114 | `fetch_and_store_metadata_persistent` example | Remove or update - method was deleted |
| Line 316 | References updating `fetch_and_store_metadata_persistent` | Remove - method no longer exists |
| Line 443 | Verification checkbox for `fetch_and_store_metadata_persistent` | Update or remove |
| Investigation section | References comparing `fetch_and_store_metadata_persistent` vs `sync_post_list` | Update - only `sync_list` exists now |

**What's still correct:**
- MetadataService owns the sync lifecycle - still true
- `refresh()` and `load_more()` API - still accurate
- Repository layer design - still accurate

**Recommendation:** Update to remove references to deleted methods. The core design is still valid.

### 3. `metadata_collection_flow.txt` - REVIEW NEEDED

This appears to be an older design document. Should be reviewed to determine if it's still relevant or should be archived.

### 4. `load_items_state_fix.md` - LIKELY OUTDATED

Bug fix documentation. The fix may still apply but should be verified against current implementation.

## Files to Delete (Already Done)

- `metadata_collection_composition.md` - Implementation doc, replaced by `metadata_collection_architecture.md`

## Summary of Code Changes That Affect Docs

1. **Renamed:** `MetadataCollection<F>` → `MetadataCollectionCore` (no generic)
2. **Deleted:** `MetadataFetcher` trait
3. **Deleted:** `PersistentPostMetadataFetcherWithEditContext`
4. **Deleted:** `PostService::fetch_and_store_metadata_persistent()`
5. **Moved:** Sync logic from core to `PostMetadataCollectionWithEditContext`
6. **Moved:** Filter from core to entity-specific collection

## Recommended Actions

1. **Immediate:** Update `metadata_collection.md` architecture diagram and struct definitions
2. **Immediate:** Update `metadata_service_orchestration.md` to remove `fetch_and_store_metadata_persistent` references
3. **Consider:** Archive `metadata_collection_flow.txt` if no longer relevant
4. **Consider:** Mark `load_items_state_fix.md` as historical
5. **Reference:** Point readers to `metadata_collection_architecture.md` for design rationale
