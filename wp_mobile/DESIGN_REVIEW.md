# wp_mobile Design Review - Concerns & Resolutions

This document captures design concerns raised during initial design discussions and their resolutions.

---

## 1. Database Read Performance

### Concern
Every `entity.data()` call reads from SQLite. In a list with 100+ items being rendered, this could mean 100+ sequential database queries during scrolling, potentially causing performance issues.

### Discussion
The design uses trait abstraction for data access:
```rust
trait DataReader<T>: Send + Sync {
    fn read(&self, id: i64) -> Option<T>;
}
```

This allows multiple implementation strategies:
- **V1 (Simple)**: Direct SQLite reads via `cache.select_by_id(id)`
- **V2 (Optimized)**: In-memory cache with batch prefetching
- **V3 (Collection-aware)**: Collection triggers batch loads, entities read from memory

### Resolution
**Status: Acceptable with validation plan**

The trait abstraction provides the necessary escape hatch for optimization. The approach is:
1. Start with simple direct DB reads
2. Benchmark with realistic data (1000+ cached posts, 100 entity accesses)
3. If performance is inadequate, implement optimized `DataReader` without changing client API
4. SQLite indexed lookups are extremely fast; optimization may not be needed

**Key insight**: The abstraction allows optimization to be added transparently without API changes. Client code using `entity.data()` remains unchanged regardless of implementation.

**Action item**: Benchmark early with realistic data volumes.

---

## 2. Observer Notification Volume

### Concern
When fetching a new page of 20 posts or updating multiple posts, there could be excessive observer callbacks:
- 20 DB row updates → 20 DatabaseDelegate notifications → 20 entity observer callbacks
- Potential UI thrashing with many rapid updates

### Discussion

**Scenario 1: Fetching new page (20 new posts)**
- New post IDs that never existed in client before
- No `SingleEntity` instances exist yet for these IDs
- Zero observers registered
- Result: No observer callbacks fire

**Scenario 2: Updating existing post (edit from detail screen)**
- List screen has `entity(50)` with observer A
- Detail screen has `entity(50)` with observer B
- User edits → DB row updates → Both observers fire
- Result: 2 callbacks for 1 post update (both screens update correctly)

**Scenario 3: Bulk operations (rare)**
- User marks 5 posts as published
- 5 DB rows update → 5 observer callbacks
- Result: 5 UI updates (acceptable for platforms)

### Resolution
**Status: Not a problem in practice**

Observer volume is naturally limited because:
1. **No observers for new items**: Can't create `SingleEntity` without an ID, so new items have no observers until accessed
2. **Updates are typically singular**: Most operations update 1-few items, not dozens
3. **Observers are scoped**: Each observer only fires for its specific entity ID
4. **Platforms handle this**: iOS/Android UI frameworks efficiently batch small numbers of updates

**DatabaseDelegate behavior**: rusqlite's update hooks fire only on actual data changes, not no-op updates.

**Future optimization path**: If needed, could add batching at collection level or debouncing in observer implementation.

---

## 3. Multiple Context Types (Edit vs View vs Embed)

### Concern
Different WordPress contexts have different fields:
- `PostWithEditContext` - 40+ fields
- `PostWithViewContext` - ~30 fields
- `PostWithEmbedContext` - ~15 fields

These are stored in separate DB tables (`post_edit_context`, `post_view_context`, `post_embed_context`).

If `SingleEntity<PostWithEdit>(id: 123)` and `SingleEntity<PostWithEmbed>(id: 123)` are different entities, updating one won't update the other, leading to inconsistent data across screens.

**Example problem**:
1. User edits post title in detail screen (edit context)
2. Post updated in `post_edit_context` table
3. List screen shows old title (reads from `post_embed_context` table)

### Discussion

This is inherent to WordPress's API design. The contexts have fundamentally different schemas, so three approaches exist:

**Option A: Union all fields**
- Single table with all fields, heavy nullability
- Loses type safety

**Option B: Separate types (current approach)**
- Matches WordPress API reality
- Type safe
- Potential for data inconsistency

**Option C: Dynamic/runtime types**
- Loses Rust's type safety entirely

**Current approach**: Option B with smart field updates when needed.

### Resolution
**Status: Acceptable - Not a wp_mobile problem**

This is a `wp_mobile_cache` layer concern, not a `wp_mobile` design issue. The inconsistency exists regardless of the SingleEntity/Collection design.

**Solution path** (when needed, in wp_mobile_cache):
```rust
// When updating edit context
fn update_post_edit_context(post: PostWithEdit) {
    update_post_edit_context_table(post);

    // Smart mapping: also update overlapping fields in other contexts
    update_post_view_context_overlapping_fields(post);
    update_post_embed_context_overlapping_fields(post);
}
```

**Decision**:
- Most clients will stick to single context per entity type
- When multi-context usage is needed, solve case-by-case in cache layer
- No global fix exists because the problem is inherent to WordPress's API design
- Defer solving until proven necessary

---

## 4. Collection State Sharing

### Concern
If multiple collection instances represent the same query (e.g., two screens showing "all posts"), should they share state via a global KV store like `SingleEntity` does?

Sharing requires complex keys:
```rust
CollectionKey {
    entity_type: Post,
    site_id: 123,
    params_hash: hash(params),  // How to compare params for equality?
}
```

### Discussion

Collection identity is fundamentally different from entity identity:
- **Entity**: Simple - (entity_type, site_id, id)
- **Collection**: Complex - (entity_type, site_id, params, filters, sort, pagination cursor, etc.)

Two collections with slightly different params are different collections:
- Tab 1: All posts
- Tab 2: Draft posts only
- Tab 3: Posts by author X

Each needs separate state (loaded pages, cursor position, etc.).

### Resolution
**Status: Instance-owned state is cleaner**

Collections manage state per-instance, not globally:

```rust
struct Collection<T> {
    // Instance owns state
    state: CollectionState,
    loaded_page_count: usize,
    has_more_pages: bool,
}
```

**Persistence approach**: Client controls when/where to save
```rust
// On screen backgrounding
let snapshot = collection.serialize_state();
platform_storage.save("home_posts_state", snapshot);

// On restoration
if let snapshot = platform_storage.load("home_posts_state") {
    collection.restore_state(snapshot);
}
```

**Benefits**:
- Simple: State lives with instance
- Flexible: Client decides persistence strategy
- No complex key generation needed
- State is small (pagination metadata, not entity data)

**Note**: Collections still share underlying entity state via `SingleEntity` global store. Only collection-level state (pagination, loading) is per-instance.

---

## 5. UniFFI Callback Threading

### Concern
When Rust fires an observer callback from a background thread, does UniFFI automatically marshal to the main thread for UI safety? If not, clients updating UI directly in observer callbacks could crash.

### Discussion

Examined existing pattern in codebase: `WpAppNotifier` trait
```rust
pub trait WpAppNotifier: Send + Sync {
    async fn requested_with_invalid_authentication(&self, request_url: String);
}
```

UniFFI generates:
- Swift: `func requestedWithInvalidAuthentication(requestUrl: String) async`
- Kotlin: `suspend fun requestedWithInvalidAuthentication(requestUrl: String)`

The pattern is:
1. Rust calls async trait method from whatever thread (typically background)
2. UniFFI bridges to async/suspend on Swift/Kotlin side
3. **Client handles threading** - can do background work, then dispatch UI updates

**Client usage**:
```swift
entity.setObserver(MyObserver {
    async func onEntityChanged() {
        // Background thread - can do work here
        let data = entity.data()

        // Explicitly hop to main thread for UI
        await MainActor.run {
            self.updateUI(with: data)
        }
    }
})
```

### Resolution
**Status: This IS the correct pattern**

**Why background thread callbacks are better**:
1. Client has flexibility to do background work (re-read DB, process data)
2. Client controls when to switch to main thread
3. Avoids unnecessary thread hopping if client doesn't need UI updates
4. Already proven pattern in existing codebase

**This is not a footgun** - it's intentional design. Clients who need UI updates know to dispatch to main thread. Clients doing background work can stay efficient.

**For entity observers**:
```rust
trait EntityObserver<T>: Send + Sync {
    async fn on_entity_changed(&self)  // Background thread, client handles main dispatch
}
```

---

## 6. Cascading Fetch Complexity

### Concern
Transparent automatic cascading (post → tags, comment → author) introduces complexity:
- When to trigger? (always, if missing, if stale)
- How deep? (1 level, recursive)
- Error handling? (fail silently, log, retry)
- Circular dependencies? (post → tags → posts)
- Performance? (100 posts → 100 tag fetches)
- Debugging? (how to know if cascade failed vs didn't trigger)

### Discussion

The goal is to make related data "just work" transparently:
```
Fetch post 123 → Post has tag IDs [5, 12, 42]
  → Service spawns background fetch for tags
  → Post returned immediately
  → When tags load → cache updated → observers fire
```

But this is complex with many edge cases and tradeoffs between user experience, reliability, performance, and debuggability.

### Resolution
**Status: Open design question - Explicitly deferred**

This is marked as an open design question in DESIGN.md with all concerns documented.

**Decision**: Design and implement cascading **after** core SingleEntity + Service layer is proven. The foundation doesn't depend on cascading being solved.

**Working theory** (not decided):
- Cascade automatically but transparently
- Primary request never waits for cascades
- Client doesn't rely on cascaded data being immediately available
- Observers notify when cascaded data arrives
- Implementation details TBD after core is validated

---

## 7. Error State Lifecycle

### Concern
When `entity.state()` returns `EntityState::Error { message }`:
- How long does error persist?
- Does it auto-clear on next fetch?
- Or require explicit clearing?
- Where is it stored (in-memory state store)?

### Discussion

This affects entity usability:
```swift
let state = entity.state()
if case .error(let msg) = state {
    // Show error UI
    // Does error persist forever? Or clear when I retry?
}
```

### Resolution
**Status: Needs design before implementation**

Not fully designed yet, but likely approach:
- Error stored in global `EntityStateStore` (in-memory)
- Auto-cleared when new fetch attempt starts (state → Loading)
- Persisted error on app restart could be stale, should be cleared on launch
- Service layer manages error state transitions

**Action item**: Design error state lifecycle before implementing entity state management.

---

## 8. Observer Lifecycle Management

### Concern
Who is responsible for removing observers?
- Client explicit removal?
- Automatic on entity drop?
- Weak references required?

### Discussion

Potential memory leak if observers hold strong references and aren't cleaned up.

Possible patterns:
```rust
// Option A: Manual removal
let handle = entity.set_observer(observer);
// Later: handle.remove()

// Option B: Automatic cleanup
entity.set_observer(weak_observer);  // Weak ref, auto-cleaned when observer drops

// Option C: Scoped
entity.with_observer(observer, || {
    // Observer active in this scope
});
```

### Resolution
**Status: Needs design before implementation**

Not fully designed yet. Likely approach:
- Follow existing `DatabaseDelegate` pattern from wp_mobile_cache
- Observers likely use weak references or manual removal
- Collection might manage observer lifecycle for its entities

**Action item**: Review `DatabaseDelegate` implementation and follow consistent pattern.

---

## Summary

### Resolved Concerns (No Blocker)
1. ✅ **DB read performance** - Trait abstraction provides escape hatch, measure before optimizing
2. ✅ **Observer volume** - Not a problem in practice, naturally limited
3. ✅ **Multiple contexts** - Cache layer concern, not wp_mobile, defer to case-by-case
4. ✅ **Collection state sharing** - Instance-owned is cleaner approach
5. ✅ **UniFFI threading** - Background callbacks are correct pattern, client handles main dispatch

### Open Questions (Need Design)
6. ⏳ **Cascading fetches** - Complex, explicitly deferred until core proven
7. ⏳ **Error state lifecycle** - Needs design before implementation
8. ⏳ **Observer lifecycle** - Needs design before implementation

### Validation Actions
- [ ] Benchmark DB read performance with realistic data (1000+ posts, 100 accesses)
- [ ] Verify observer volume acceptable in real scrolling scenarios
- [ ] Design error state lifecycle and transitions
- [ ] Design observer lifecycle management (follow DatabaseDelegate pattern)
- [ ] Prototype SingleEntity + Service layer before tackling Collection/Cascading

---

**Conclusion**: The foundation (SingleEntity with global state, Service layer, trait abstractions) is solid and ready for prototyping. Open questions are appropriately deferred and don't undermine the core design.
