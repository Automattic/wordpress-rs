# wp_mobile Design Document

## Overview

The `wp_mobile` crate provides a unified business layer for WordPress mobile applications (iOS & Android). It bridges the networking layer (`wp_api`) and caching layer (`wp_mobile_cache`) to provide a consistent, observable, and efficient data access pattern for mobile clients.

## Design Principles

1. **Cache as bonus, not requirement**: The same `wp_api` types flow through the entire stack (network → cache → business layer → client). The cache is transparent and completely hidden from clients.

2. **Simple defaults with optimization opportunities**: Provide sensible default behavior while allowing for optimizations like smart prefetching, cascading fetches, etc.

3. **Mobile-first lifecycle**: Design for mobile app lifecycles (backgrounding, process termination, state restoration).

4. **Minimal FFI crossings**: Data should cross the FFI boundary only when needed, and as efficiently as possible.

5. **Observable by default**: Clients should be notified of data changes regardless of whether they initiated the change.

## Architecture

### Three-Layer Design

```
┌─────────────────────────────────────┐
│  Service Layer (Stateless)          │
│  PostService, CommentService, etc.  │
│                                      │
│  - Direct CRUD operations           │
│  - Network ↔ Cache bridge           │
│  - Creates SingleEntity & Collection│
└──────────────────────────────────────┘
                 │
        ┌────────┴────────┐
        ▼                 ▼
┌──────────────┐  ┌─────────────────┐
│Collection<T> │  │ SingleEntity<T> │
│(List State)  │  │ (Item State)    │
│              │  │                 │
│Stateful      │  │ Lightweight     │
│Instance-owned│  │ Observable      │
│state         │  │ Global state    │
└──────────────┘  └─────────────────┘
```

### Layer Responsibilities

#### Service Layer (e.g., `PostService`)

**Purpose**: Stateless bridge between client and underlying systems.

**Responsibilities**:
- Fetch data from network or cache
- Create, update, delete operations
- Decide when to use network vs cache
- Update cache with network responses
- Create `SingleEntity<T>` and `Collection<T>` instances

**Characteristics**:
- Mostly stateless (may use internal state store for bookkeeping like last sync times)
- Does NOT handle pagination
- Does NOT track loading states per item
- Simple, direct API

**Example API**:
```rust
impl PostService {
    // Direct operations
    fn get_post(site: DbSite, id: PostId, force: bool) -> Result<Post>
    fn create_post(site: DbSite, params: PostCreateParams) -> Result<Post>
    fn update_post(site: DbSite, id: PostId, params: PostUpdateParams) -> Result<Post>
    fn delete_post(site: DbSite, id: PostId) -> Result<()>

    // Create managed instances
    fn get_entity(site: DbSite, id: PostId) -> SingleEntity<Post>
    fn create_collection(site: DbSite, params: PostListParams) -> Collection<Post>
}
```

#### SingleEntity\<T\>

**Purpose**: Lightweight handle to a single entity with observable state and metadata.

**Design**:
```rust
struct SingleEntity<T> {
    id: i64,
    site_id: i64,
    // Note: entity_type is T itself, no separate enum needed

    // Shared references to global resources (via traits)
    state_reader: Arc<dyn StateReader>,
    data_reader: Arc<dyn DataReader<T>>,
}
```

**Key Characteristics**:

1. **Lightweight**: Just an ID + references to global stores. Creating multiple instances is cheap.

2. **PartialEq by ID**: Two entities with the same ID are considered equal, even if different instances.

3. **Data from DB**: Every call to `data()` reads from the database (SQLite is fast enough for this).

4. **State from global store**: State is read from a global, in-memory `EntityStateStore`.

5. **Observable**: Can attach observers that fire when the underlying data changes.

**API**:
```rust
impl SingleEntity<T> {
    /// Get current data (reads from cache/DB)
    fn data(&self) -> Option<T>

    /// Get current state (reads from state store)
    fn state(&self) -> EntityState

    /// When was this last fetched? (reads from DB metadata)
    fn last_fetched_at(&self) -> Option<DateTime>

    /// Force refresh from network
    fn refresh(&self) -> Result<()>

    /// Observe changes to this entity
    fn set_observer(&self, observer: Arc<dyn EntityObserver<T>>)
    fn remove_observer(&self)
}

enum EntityState {
    Empty,      // Not in cache
    Fresh,      // In cache and recent
    Stale,      // In cache but old
    Loading,    // Currently fetching
    Error { message: String },
}
```

**Observer Pattern**:

```rust
trait EntityObserver<T>: Send + Sync {
    /// Called when entity data changes
    /// Observer should re-read data() to get updated value
    fn on_entity_changed(&self)
}
```

The observer doesn't receive the new data directly - it's just a notification to re-read `data()`. This keeps it simple and avoids data crossing FFI unnecessarily.

**How observers work**:
1. Entity registers with cache's existing `DatabaseDelegate` system
2. Filter to only changes for this specific entity's ID
3. When DB row changes → observer fires
4. Client re-reads `data()` to get updated value

**Use Cases**:
- Post detail screen: `let entity = postService.get_entity(id: 123)`
- Individual items accessed from collections
- Any single-item scenario where you want to observe changes

#### Collection\<T\>

**Purpose**: Stateful list manager with pagination, prefetching, and loading state.

### Design Status: **OPEN - Needs Design Work**

**High-level concept**:
```rust
struct Collection<T> {
    // Instance owns its state (not global like SingleEntity)
    state: CollectionState,
    items: Vec<SingleEntity<T>>,  // Loaded items as entities
    // ... pagination state, etc.
}

enum CollectionState {
    LoadingFirstPage,
    LoadingMorePages,
    FetchedAllPages,
    Error { message: String },
}
```

**Key Characteristics**:
- Stateful (unlike Service and SingleEntity)
- Owns/manages a list of `SingleEntity<T>` instances
- Handles pagination automatically (or semi-automatically)
- Can prefetch as user scrolls
- Maps well to platform list abstractions (UITableView, RecyclerView)

**State Management Approach (Current Thinking)**:

Unlike `SingleEntity` which uses global state, Collections should manage state per-instance:

**Reasoning**:
- Collection identity is complex: (entity_type, site_id, params, filters, sort order, etc.)
- Too many variations to use global state store with complex keys
- Each collection instance represents different query/screen
- State is small (pagination metadata, not actual entity data)

**Persistence approach**:
```rust
impl Collection<T> {
    // Serialize just the metadata
    fn serialize_state(&self) -> CollectionStateSnapshot

    // Restore from saved state
    fn restore_state(&mut self, snapshot: CollectionStateSnapshot)
}

// Client decides when/where to save
// e.g., on iOS viewWillDisappear
let state = collection.serialize_state();
UserDefaults.save("home_post_list_state", state);

// On restoration
if let state = UserDefaults.load("home_post_list_state") {
    collection.restore_state(state);
}
```

**Benefits**:
- Simple: State lives with instance
- Flexible: Client controls persistence
- No complex global key generation
- State is lightweight (page count, cursor, flags - not entity data)

**Tentative API** (subject to change):
```rust
impl Collection<T> {
    fn count() -> usize
    fn object_at_index(index: usize) -> SingleEntity<T>
    fn refresh(force: bool) -> Result<()>
    fn state() -> CollectionState
    fn set_observer(observer: Arc<dyn CollectionObserver>)

    // Persistence helpers
    fn serialize_state(&self) -> CollectionStateSnapshot
    fn restore_state(&mut self, snapshot: CollectionStateSnapshot)
}
```

**Use Cases**:
- Post list screen
- Comment list screen
- Any paginated list UI

**Major Open Questions**:
1. **Pagination mechanism**:
   - Automatic prefetch when near end?
   - Explicit `load_next_page()` calls?
   - Platform-driven prefetch hints?

2. **Entity creation timing**:
   - Create all entities upfront when page loads?
   - Lazy creation on `object_at_index()` access?
   - Batch creation for visible range?

3. **Filters & search**:
   - New filter = new collection instance?
   - Update existing collection's params?
   - How to handle incremental search?

4. **Observer patterns**:
   - Collection-level observer for "page loaded" events?
   - Individual entity observers for item updates?
   - Both?

5. **State snapshot contents**:
   - What exactly gets serialized? (page count, loaded IDs, cursor position?)
   - How to handle cache invalidation on restore?
   - Should we re-validate on restore or trust saved state?

6. **Multi-collection coordination**:
   - If two screens show same posts but different filters, how to handle?
   - Shared entities (via SingleEntity) but separate collection state?

**Decision**: Design and implement Collection after SingleEntity + Service layer is proven and working.

---

## Global State Management

### EntityStateStore

**Purpose**: Global, in-memory store tracking the state of all entities.

**Design**:
```rust
struct EntityStateStore {
    states: RwLock<HashMap<EntityKey, EntityState>>,
}

struct EntityKey {
    entity_type: EntityType,
    site_id: i64,
    entity_id: i64,
}

enum EntityType {
    Post,
    Comment,
    Term,
    Media,
    // etc.
}
```

**Characteristics**:

1. **Global/Singleton**: One instance shared across all services and entities.

2. **In-memory**: Fast reads/writes, automatically cleared on app restart.

3. **Optional persistence**: Can be saved/restored on app lifecycle events:
   ```rust
   // On iOS: applicationDidEnterBackground
   // On Android: onPause/onStop
   fn save_state() {
       let serialized = state_store.serialize();
       platform_storage.save(serialized);
   }

   // On app launch
   fn restore_state() {
       if let Some(serialized) = platform_storage.load() {
           state_store.restore(serialized);
           state_store.clear_transient();  // Clear Loading/Error states
       }
   }
   ```

4. **Transient state cleared on restart**: `Loading` and `Error` states are cleared on launch to avoid showing stale "loading" indicators.

5. **Persistent state derived from DB**: `Fresh`/`Stale` states can be derived from the DB's `last_fetched_at` field, so they don't strictly need persistence.

**Access Pattern**:

```rust
trait StateReader: Send + Sync {
    fn get_state(&self, key: EntityKey) -> EntityState;
}

trait StateWriter: Send + Sync {
    fn set_state(&self, key: EntityKey, state: EntityState);
    fn clear_state(&self, key: EntityKey);
}

// SingleEntity gets a StateReader (read-only)
// Services get a StateWriter (can modify)
```

This prevents entities from arbitrarily modifying global state.

---

## Data Flow

### Fetch Flow (e.g., fetching a post)

```
Client calls: postService.get_entity(id: 123)
                    ↓
        PostService creates SingleEntity
                    ↓
Client calls: entity.data()
                    ↓
          Entity reads from cache DB
                    ↓
        Is data fresh? (check last_fetched_at)
                    ↓
            ┌───────┴───────┐
          YES              NO
            ↓                ↓
    Return cached      Optionally trigger
        data           background refresh
                            ↓
                    Fetch from network
                            ↓
                    Update cache DB
                            ↓
                DatabaseDelegate fires
                            ↓
                Entity observer fires
                            ↓
                Client re-reads data()
```

### Observer Flow

```
Network response received
        ↓
Service updates cache
        ↓
Cache writes to DB (wp_mobile_cache)
        ↓
DatabaseDelegate detects change
        ↓
EntityStateStore tracks change
        ↓
All SingleEntity instances for this ID are notified
        ↓
Each entity's observers fire
        ↓
Clients re-read data() to get fresh data
```

---

## Fetch Strategy

### Default Strategy (Single Smart Strategy)

Rather than multiple configurable strategies, implement one smart default:

1. **Check cache**: Look for entity in cache DB
2. **If cached and fresh**: Return immediately
3. **If cached but stale**:
   - Return cached data (UI shows immediately)
   - Optionally: Trigger background refresh
   - Observer fires when refresh completes
4. **If not cached**: Fetch from network synchronously
5. **Force refresh**: Optional `force: bool` parameter bypasses cache (pull-to-refresh)

### Staleness Policy

Entities have different staleness thresholds:
- Posts: 5-15 minutes (frequently updated)
- Terms (tags/categories): 1 hour (rarely change)
- Users: 1 day (very stable)
- Site settings: Even longer

**Design Question**: How to configure per-entity staleness?
- Per-service constants?
- Global config with per-entity overrides?

---

## Cascading Fetches

### The Problem

When fetching a post, we discover it has tags with IDs `[5, 12, 42]`. Should we fetch those tags automatically?

### Design Status: **OPEN - Needs Design Work**

The goal is to support **transparent, automatic cascading** where:
- Service layer fetches related entities (post → tags, comment → author, etc.) automatically
- Client doesn't rely on cascading for correctness (must handle missing related data)
- Cascading happens in background/async, doesn't block primary request
- Observer pattern notifies when related entities become available

### Key Design Questions (Unanswered)

1. **When to cascade?**
   - Always when fetching a post?
   - Only if related entities are missing from cache?
   - Only if related entities are stale?
   - Different rules for different relationships?

2. **How deep to cascade?**
   - Level 1: Post → tags only
   - Level 2: Post → tags → parent tags
   - Recursive until all related data fetched?

3. **Which relationships to cascade?**
   - Post → tags/categories (definitely)
   - Post → author (maybe)
   - Post → featured media (maybe)
   - Comment → post (probably not - could create cycles)
   - How to avoid circular dependencies?

4. **Error handling**
   - Primary fetch succeeds, cascade fails: Silent? Logged? Observable?
   - Partial cascades: Some tags fetch, others fail?
   - Should errors be visible to client at all?

5. **Performance & throttling**
   - Fetching 100 posts → 100 cascade requests for tags: How to batch?
   - Rate limiting cascade requests?
   - Priority: Primary vs cascaded fetches?

6. **Observability**
   - How does client know cascaded data has arrived?
   - Entity observers fire when tags become available?
   - Collection-level "related data loaded" event?

7. **Debugging**
   - When tags don't show up, how to know if cascading failed vs didn't trigger?
   - Logging strategy?
   - Developer visibility into what's being cascaded?

### Working Theory (Not Yet Decided)

Transparent cascading where:
- Service layer triggers background fetches for related entities
- Primary request completes immediately (doesn't wait for cascades)
- When cascade completes → cache updated → observers fire
- Client handles both "data available immediately" and "data arrives later" via observers

Example flow:
```
Client: postService.get_entity(id: 123)
  ↓
Service: Fetch post 123
  ↓
Post has tag IDs [5, 12, 42]
  ↓
Service: Return post immediately
Service: Spawn background: fetch tags [5, 12, 42] (if missing/stale)
  ↓
Client: entity.data() returns post with tag IDs
Client: Sets observer on entity
  ↓
Background: Tag fetches complete
  ↓
Cache: Updated with tags
  ↓
Observers: Fire for tag entities (if any exist)
  ↓
Client: Can now fetch tag entities if needed
```

**This approach needs validation** - implementation details TBD.

### Why This is Complex

The challenge is balancing:
- **User experience**: Related data should "just work" when possible
- **Reliability**: Primary data must not fail because related data fails
- **Performance**: Don't create cascade storms
- **Predictability**: Client shouldn't be confused about what's available when
- **Debuggability**: When things go wrong, must be traceable

**Decision**: Defer detailed cascading design until core SingleEntity + Service layer is proven.

---

## Threading & Async

### Current Approach: Async Functions

Following the existing `wp_api` pattern, use async functions:

```rust
impl PostService {
    async fn get_post(...) -> Result<Post>
    async fn create_post(...) -> Result<Post>
}
```

**Why async**:
1. Matches existing `wp_api` patterns
2. Clients already handle async in Swift/Kotlin
3. Non-blocking for network operations
4. UniFFI has support for async (though still maturing)

**Client usage**:
```swift
// iOS
Task {
    let post = try await postService.getPost(id: 123)
}

// Or with entity
let entity = postService.getEntity(id: 123)
entity.setObserver { [weak self] in
    self?.updateUI(with: entity.data())
}
Task {
    try await entity.refresh()
}
```

---

## Type System & Contexts

### Using wp_api Types Directly

The cache returns the same types as the API (`AnyPostWithEditContext`, `AnyPostWithViewContext`, etc.). The business layer uses these directly without intermediate types.

**For Collections**: May use lighter contexts for list views.
```rust
// List screen uses embed context (minimal data)
let collection = postService.create_collection_with_embed_context(params);

// Detail screen uses edit context (full data)
let entity = postService.get_entity_with_edit_context(id: 123);
```

**Design Question**: How do contexts work with SingleEntity?
- Are `SingleEntity<PostWithEdit>` and `SingleEntity<PostWithEmbed>` different entities for the same post ID?
- Or is there one entity that can be upgraded (`.refresh_with_context(EditContext)`)?

---

## Outstanding Design Questions

### Collection Design
1. How exactly does pagination work?
   - Automatic prefetch when near end?
   - Explicit `load_next_page()` calls?
   - Platform-driven prefetch hints?

2. How is collection state managed?
   - When collection is `LoadingFirstPage`, what's the state of individual entities?
   - Does collection create entities eagerly or lazily?

3. How do filters/search work?
   - New filter = new collection instance?
   - Or update existing collection?

4. Collection-level observers vs entity-level observers?
   - Does collection observer fire for "page loaded"?
   - Do entity observers fire for individual item changes?

### Context & Type Questions
1. How do different contexts (`edit`, `view`, `embed`) work with `SingleEntity`?
2. Should there be intermediate types for lists, or always use `wp_api` types?
3. How to handle context upgrades (list with embed → detail with edit)?

### Staleness Configuration
1. Should staleness be configurable per entity type?
2. Global config with per-entity defaults vs per-service constants?
3. Can clients customize staleness, or is it internal to business layer?

### Lifecycle & State Persistence
1. When exactly should state be saved/restored?
2. What subset of state should be persisted (just Loading/Error, or all state)?
3. How to handle restoration failures?

---

## Implementation Plan

### Phase 1: Core SingleEntity (Current Focus)
- [ ] Define `SingleEntity<T>` structure
- [ ] Implement `EntityStateStore` (in-memory)
- [ ] Wire up observer pattern with existing `DatabaseDelegate`
- [ ] Create basic `PostService` with direct operations
- [ ] Test entity creation, observation, state management

### Phase 2: Service Layer
- [ ] Implement fetch logic (cache-first with staleness)
- [ ] Add force refresh capability
- [ ] Handle network errors
- [ ] Add other services (CommentService, TermService, etc.)

### Phase 3: Collection (Deferred)
- [ ] Design collection pagination strategy
- [ ] Implement `Collection<T>` structure
- [ ] Add prefetching logic
- [ ] Handle collection state management
- [ ] Test with platform list views

### Phase 4: Optimizations (Future)
- [ ] Smart cascading for related entities
- [ ] State persistence/restoration
- [ ] Per-entity staleness policies
- [ ] Background refresh strategies

---

## Notes

- This design prioritizes simplicity and correctness over premature optimization
- Start with basic patterns, add complexity only when needed
- Mobile lifecycle is a first-class concern
- The cache is completely transparent to clients
- Observer pattern is the primary way clients stay updated
