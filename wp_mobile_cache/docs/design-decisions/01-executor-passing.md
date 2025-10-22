# Design Decision 1: Explicit Executor Passing

> **Last Updated:** 2025-10-21

## Decision

Pass `executor` as a parameter to each repository method rather than storing it in the repository.

## Context

Repository pattern implementations often face a choice:
1. Store the executor (connection/transaction) in the repository
2. Pass the executor to each method call

Example comparison:

```rust
// Option 1: Store executor (rejected)
struct PostRepository<'conn> {
    conn: &'conn Connection,
}

impl<'conn> PostRepository<'conn> {
    pub fn select_by_id(&self, id: RowId) -> Result<Post> {
        // Use self.conn
    }
}

// Option 2: Pass executor (chosen)
struct PostRepository;

impl PostRepository {
    pub fn select_by_id(&self, executor: &impl QueryExecutor, id: RowId) -> Result<Post> {
        // Use executor parameter
    }
}
```

## Rationale

### Simplicity

**No lifetime complexity:**
- Repository is zero-sized, no lifetime parameters needed
- No borrow checker issues with repository lifetime vs connection lifetime
- Straightforward to use and understand

```rust
// ✅ Simple - no lifetimes
let repo = PostRepository;
let post = repo.select_by_id(&conn, RowId(42))?;
```

vs.

```rust
// ❌ Complex - lifetime management
let repo = PostRepository { conn: &conn };
let post = repo.select_by_id(RowId(42))?;
// Must worry about repo lifetime tied to conn
```

### Flexibility

**Clients can use different executors per call:**

```rust
let repo = PostRepository;
let site = DbSite { row_id: RowId(1) };

// Use connection for simple queries
let post = repo.select_by_rowid(&conn, &site, RowId(42))?;

// Use transaction for atomic operations
let mut conn = Connection::open("cache.db")?;
let tx = conn.transaction()?;
repo.insert(&tx, &new_post, &site)?;
repo.insert(&tx, &another_post, &site)?;
tx.commit()?;
```

With stored executor, switching between connection and transaction requires creating new repository instances.

### YAGNI (You Aren't Gonna Need It)

**Clients can build convenience wrappers if they want them:**

```rust
// Optional convenience wrapper (clients can create if needed)
struct PostRepositoryWithExecutor<E> {
    executor: E,
    repo: PostRepository,
    site: DbSite,
}

impl<E: QueryExecutor> PostRepositoryWithExecutor<E> {
    pub fn select_by_rowid(&self, rowid: RowId)
        -> Result<DbAnyPostWithEditContext, SqliteDbError> {
        self.repo.select_by_rowid(&self.executor, &self.site, rowid)
    }
}
```

**The library doesn't force a specific usage pattern** - it provides the building blocks.

### Least Committal

**Easy to add wrappers later, hard to remove complexity:**

- Starting simple: ✅ Can add convenience later
- Starting complex: ❌ Can't easily simplify without breaking changes

This design allows evolution without breaking existing code.

## Example Usage

### Basic Operations

```rust
let conn = Connection::open("cache.db")?;
let repo = PostRepository;
let site = DbSite { row_id: RowId(1) };

// Each call explicitly passes executor
let post = repo.select_by_rowid(&conn, &site, RowId(42))?;
let all_posts = repo.select_all(&conn, &site)?;
let author_posts = repo.select_by_author(&conn, &site, UserId(1))?;
```

### Transaction Operations

```rust
let mut conn = Connection::open("cache.db")?;
let repo = PostRepository;
let site = DbSite { row_id: RowId(1) };

// Explicit transaction management
let tx = conn.transaction()?;

let rowid1 = repo.insert(&tx, &post1, &site)?;
let rowid2 = repo.insert(&tx, &post2, &site)?;

tx.commit()?;
```

### Mixed Operations

```rust
// Read from connection
let post = repo.select_by_rowid(&conn, &site, RowId(1))?;

// Write in transaction
let tx = conn.transaction()?;
let updated_rowid = repo.upsert(&tx, &site, &updated_post)?;
tx.commit()?;

// Read from connection again
let post = repo.select_by_rowid(&conn, &site, RowId(1))?;
```

## Alternatives Considered

### Alternative 1: Store Executor in Repository

```rust
struct PostRepository<'conn> {
    conn: &'conn Connection,
    site: DbSite,
}

impl<'conn> PostRepository<'conn> {
    pub fn select_by_id(&self, id: RowId) -> Result<Post> {
        // Use self.conn
    }
}
```

**Rejected because:**
- Lifetime complexity throughout codebase
- Cannot easily switch between Connection and Transaction
- Harder to use with transactions
- More restrictive API

### Alternative 2: Store Owned Connection

```rust
struct PostRepository {
    conn: Connection,
    site: DbSite,
}

impl PostRepository {
    pub fn select_by_id(&self, id: RowId) -> Result<Post> {
        // Use &self.conn
    }
}
```

**Rejected because:**
- Repository owns the connection (unusual ownership model)
- Cannot share connection between repositories
- Complicates transaction handling
- Mutable access issues (`&mut self` required for writes)

### Alternative 3: Trait Object Approach

```rust
struct PostRepository {
    executor: Box<dyn QueryExecutor>,
    site: DbSite,
}
```

**Rejected because:**
- Runtime overhead (virtual dispatch)
- More complex to construct
- Less flexibility with trait bounds
- No significant benefit over explicit passing

## Trade-offs

### Advantages

✅ **Zero lifetime complexity** - Repository has no lifetime parameters
✅ **Maximum flexibility** - Any executor can be used per call
✅ **Clear ownership** - No confusion about who owns what
✅ **Easy to understand** - Explicit parameter passing is straightforward
✅ **Future-proof** - Easy to add convenience wrappers later

### Disadvantages

❌ **More verbose** - Must pass executor to every call
❌ **Repetitive** - Executor appears in many call sites
❌ **No default executor** - Cannot have "default connection" behavior

However, the disadvantages can be mitigated by client-side wrapper types if needed.

## Related Decisions

- [Zero-Sized Repositories](03-zero-sized-repos.md) - Why repositories have no fields
- [Associated Types](02-associated-types.md) - Repository type design
- [Split Traits](08-split-traits.md) - QueryExecutor vs TransactionManager

## References

- [Repository Pattern](https://martinfowler.com/eaaCatalog/repository.html)
- [YAGNI Principle](https://en.wikipedia.org/wiki/You_aren%27t_gonna_need_it)

## See Also

- [Core Traits](../architecture/core-traits.md) - `QueryExecutor` trait definition
- [Usage Examples](../usage-examples.md) - Practical examples
