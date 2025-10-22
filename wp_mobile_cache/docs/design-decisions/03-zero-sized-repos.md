# Design Decision 3: Zero-Sized Repositories

> **Last Updated:** 2025-10-21

## Decision

Repositories are zero-sized structs with no fields.

## Context

Repositories can be implemented in different ways:

```rust
// Option 1: Zero-sized struct (chosen)
pub struct PostRepository;

// Option 2: Struct with fields (rejected)
pub struct PostRepository {
    // config or state fields
}

// Option 3: Trait object (rejected)
pub trait PostRepository {
    // methods
}
```

## Rationale

### Zero-Cost Abstraction

**No runtime overhead:**

```rust
pub struct PostRepository;  // Size: 0 bytes

let repo = PostRepository;
std::mem::size_of_val(&repo);  // Returns: 0
```

**Compiler optimizations:**
- Calling methods on zero-sized types has zero cost
- No memory allocation needed
- No copying or moving costs
- Equivalent to calling free functions

**Benchmark equivalent:**
```rust
// These have identical performance:
repo.select_by_id(&conn, &site, id)?;
select_by_id(&conn, &site, id)?;  // Free function
```

### Stateless Design

**All operations are stateless:**

Repositories don't need to maintain state because:
- Database executor is passed explicitly
- Site is passed explicitly
- No configuration needed
- All data flows through parameters

```rust
impl PostRepository {
    pub fn select_by_rowid(
        &self,  // &self is actually zero bytes
        executor: &impl QueryExecutor,
        site: &DbSite,
        rowid: RowId,
    ) -> Result<DbAnyPostWithEditContext, SqliteDbError> {
        // All data comes from parameters
    }
}
```

### Singleton-Friendly

**Clients can create singletons if desired:**

```rust
// As a local variable (no cost)
let repo = PostRepository;

// As a static (no runtime initialization needed)
static POST_REPO: PostRepository = PostRepository;

// As a struct field (no size impact)
struct Cache {
    post_repo: PostRepository,  // Adds 0 bytes
    user_repo: UserRepository,  // Adds 0 bytes
}
```

**But singletons aren't required:**

```rust
// Can create on-demand with no cost
fn get_posts() -> Result<Vec<Post>> {
    let repo = PostRepository;  // No allocation
    repo.select_all(&conn, &site)
}
```

### Simple Construction

**No constructor needed:**

```rust
// ✅ Direct construction
let repo = PostRepository;

// No need for:
// ❌ PostRepository::new()
// ❌ PostRepository::builder()
// ❌ PostRepository { config: ... }
```

**Clear intent:**

```rust
// Obvious what this is
let repo = PostRepository;
repo.insert(&conn, &post, &site)?;
```

## Example Usage

### Local Usage

```rust
fn cache_posts(posts: Vec<AnyPostWithEditContext>) -> Result<()> {
    let repo = PostRepository;  // Zero-sized, zero cost
    let conn = Connection::open("cache.db")?;
    let site = DbSite { row_id: RowId(1) };

    for post in posts {
        repo.insert(&conn, &post, &site)?;
    }

    Ok(())
}
```

### Static Repository

```rust
static POST_REPO: PostRepository = PostRepository;

fn get_all_posts() -> Result<Vec<DbAnyPostWithEditContext>> {
    let conn = Connection::open("cache.db")?;
    let site = DbSite { row_id: RowId(1) };
    POST_REPO.select_all(&conn, &site)
}
```

### Multiple Repositories

```rust
struct CacheManager {
    post_repo: PostRepository,   // 0 bytes
    user_repo: UserRepository,   // 0 bytes
    media_repo: MediaRepository, // 0 bytes
}

impl CacheManager {
    fn new() -> Self {
        Self {
            post_repo: PostRepository,
            user_repo: UserRepository,
            media_repo: MediaRepository,
        }
    }
}

// Sizeof CacheManager is still 0 bytes!
```

### Generic Functions

```rust
fn process_repository<R: Repository>(repo: R) {
    // R is zero-sized, passed by value with no cost
}

// Called with zero-sized value
process_repository(PostRepository);
```

## Alternatives Considered

### Alternative 1: Repository with Configuration

```rust
pub struct PostRepository {
    table_name: &'static str,
    // other config
}

impl PostRepository {
    pub fn new() -> Self {
        Self {
            table_name: "posts_edit_context",
        }
    }
}
```

**Why rejected:**
- Adds runtime overhead (memory allocation)
- Configuration is compile-time constant anyway
- Can use `const` if needed:
  ```rust
  impl PostRepository {
      const TABLE_NAME: &'static str = "posts_edit_context";
  }
  ```

### Alternative 2: Repository with State

```rust
pub struct PostRepository {
    cache: HashMap<RowId, CachedPost>,
}
```

**Why rejected:**
- Caching should be explicit, not hidden
- Violates single responsibility principle
- Complicates thread safety
- State belongs in a separate caching layer if needed

### Alternative 3: Trait Object

```rust
pub trait PostRepository {
    fn select_by_id(&self, ...) -> Result<Post>;
}

pub struct SqlitePostRepository;

impl PostRepository for SqlitePostRepository {
    // ...
}
```

**Why rejected:**
- Requires dynamic dispatch (performance overhead)
- More complex to use (Box<dyn PostRepository>)
- No current need for multiple implementations
- Can add trait later if needed without breaking changes

### Alternative 4: Module with Free Functions

```rust
pub mod post_repository {
    pub fn select_by_id(...) -> Result<Post> { }
    pub fn insert(...) -> Result<RowId> { }
}
```

**Why rejected:**
- Loses ability to use traits
- Cannot write generic code over repositories
- Less idiomatic Rust
- However: Equivalent performance to zero-sized struct

## When Zero-Sized Structs Make Sense

### ✅ Use Zero-Sized Structs When:

- Type has no state
- All operations are stateless
- Type is used for namespacing methods
- Performance is critical (zero overhead)
- Type implements traits

**Examples:**
- Repositories (our use case)
- Strategy pattern implementations
- Type-level markers
- Unit-like enums

### ❌ Don't Use Zero-Sized Structs When:

- Type needs to maintain state
- Configuration varies at runtime
- Multiple instances with different behavior needed
- State machine implementations

## Memory Layout Verification

```rust
#[test]
fn repository_is_zero_sized() {
    assert_eq!(std::mem::size_of::<PostRepository>(), 0);
    assert_eq!(std::mem::size_of::<UserRepository>(), 0);

    // Verify no allocation
    let repo = PostRepository;
    let ptr = &repo as *const _;
    // Zero-sized types have special pointer handling
}
```

## Related Decisions

- [Executor Passing](01-executor-passing.md) - Why executors aren't stored
- [Associated Types](02-associated-types.md) - Repository type design
- [Minimal Abstraction](04-minimal-abstraction.md) - Keeping things simple

## References

- [Rust RFC: Zero-Sized Types](https://rust-lang.github.io/rfcs/1521-copy-clone-semantics.html)
- [Rust Book: Tuple Structs Without Named Fields](https://doc.rust-lang.org/book/ch05-01-defining-structs.html#unit-like-structs-without-any-fields)

## See Also

- [Core Traits](../architecture/core-traits.md) - Repository trait definition
- [Usage Examples](../usage-examples.md) - Practical usage patterns
