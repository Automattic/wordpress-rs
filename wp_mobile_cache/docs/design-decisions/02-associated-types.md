# Design Decision 2: Associated Types Over Generic Parameters

> **Last Updated:** 2025-10-22

> **Historical Note:** The `Repository` trait referenced in this document was removed as a premature abstraction (as of 2025-10-22). Each repository now defines its own API directly. This document is preserved for educational purposes to explain the associated types pattern, should a shared repository trait be needed in the future.

## Decision

Use `type Entity = ...` (associated type) instead of `Repository<T>` (generic parameter).

## Context

Rust provides two ways to associate a type with a trait:

```rust
// Option 1: Associated type (chosen)
trait Repository {
    type Entity: DbEntity;
}

impl Repository for PostRepository {
    type Entity = AnyPostWithEditContext;
}

// Option 2: Generic parameter (rejected)
trait Repository<T: DbEntity> {
    // ...
}

impl Repository<AnyPostWithEditContext> for PostRepository {
    // ...
}
```

## Rationale

### 1-to-1 Relationship

**Each repository is for exactly one entity type:**

```rust
// ✅ Clear 1-to-1 relationship
pub struct PostRepository;
impl Repository for PostRepository {
    type Entity = AnyPostWithEditContext;
}

pub struct UserRepository;
impl Repository for UserRepository {
    type Entity = User;
}
```

**Why this matters:**
- A repository is dedicated to managing one type of entity
- No use case for a single struct implementing `Repository` for multiple types
- Associated types enforce this natural constraint

### Cleaner Syntax

**Type appears in context, not in every reference:**

```rust
// ✅ With associated type
let repo = PostRepository;
repo.upsert(&conn, &post, &site)?;

// ❌ With generic parameter
let repo = PostRepository;
repo.insert::<AnyPostWithEditContext>(&conn, &post, &site)?;
// OR
let repo: Repository<AnyPostWithEditContext> = PostRepository;
```

**In function signatures:**

```rust
// ✅ With associated type
fn process_repo(repo: &impl Repository) {
    // Entity type accessible via repo::Entity
}

// ❌ With generic parameter
fn process_repo<T: DbEntity>(repo: &impl Repository<T>) {
    // Must specify T separately
}
```

### Better Ergonomics

**Type inference works naturally:**

```rust
impl Repository for PostRepository {
    type Entity = AnyPostWithEditContext;

    fn upsert(&self, executor: &impl QueryExecutor,
              item: &Self::Entity, site: &DbSite) -> Result<RowId> {
        // Compiler knows Self::Entity is AnyPostWithEditContext
        item.insert_into_db(executor, site)
    }
}
```

**Type can be accessed via path:**

```rust
// Access the associated type
type PostEntity = <PostRepository as Repository>::Entity;

// Use in function bounds
fn work_with_posts<R>(repo: &R)
where
    R: Repository<Entity = AnyPostWithEditContext>
{
    // ...
}
```

## Example Usage

### Repository Implementation

```rust
pub struct PostRepository;

impl Repository for PostRepository {
    type Entity = AnyPostWithEditContext;
}

impl PostRepository {
    // Methods can use Self::Entity or directly reference the type
    pub fn select_by_rowid(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        rowid: RowId,
    ) -> Result<DbAnyPostWithEditContext, SqliteDbError> {
        // Implementation uses AnyPostWithEditContext
    }
}
```

### Client Code

```rust
fn cache_posts(posts: Vec<AnyPostWithEditContext>) -> Result<()> {
    let conn = Connection::open("cache.db")?;
    let repo = PostRepository;
    let site = DbSite { row_id: RowId(1) };

    for post in posts {
        // Clean call - no type annotations needed
        repo.upsert(&conn, &post, &site)?;
    }

    Ok(())
}
```

### Generic Functions

```rust
// Can still write generic functions over repositories
fn count_entities<R: Repository>(
    repo: &R,
    executor: &impl QueryExecutor,
) -> Result<usize> {
    // Access entity type via R::Entity
    todo!()
}
```

## Alternatives Considered

### Alternative 1: Generic Parameter

```rust
trait Repository<T: DbEntity> {
    fn upsert(&self, executor: &impl QueryExecutor,
              item: &T, site: &DbSite) -> Result<RowId>;
}

impl Repository<AnyPostWithEditContext> for PostRepository {
    fn upsert(&self, executor: &impl QueryExecutor,
              item: &AnyPostWithEditContext, site: &DbSite) -> Result<RowId> {
        // ...
    }
}
```

**Why rejected:**
- More verbose at call sites
- Could theoretically implement multiple times for same struct (confusing)
- Harder to reference in where clauses
- No advantage for our use case

### Alternative 2: No Trait, Just Concrete Types

```rust
// No Repository trait at all
pub struct PostRepository;

impl PostRepository {
    pub fn upsert(
        &self,
        executor: &impl QueryExecutor,
        item: &AnyPostWithEditContext,
        site: &DbSite,
    ) -> Result<RowId> {
        // ...
    }
}
```

**Why rejected:**
- Loses shared interface for common operations
- Cannot write generic code over repositories
- More code duplication
- However: This is valid for methods unique to specific repositories

**Note:** We use both approaches:
- `Repository` trait for common operations (insert, insert_batch)
- Direct implementation for entity-specific methods (select_by_post_id)

## When to Use Which

### Use Associated Types When:

✅ There's a 1-to-1 relationship between trait implementor and type
✅ The type is fundamental to the trait's purpose
✅ You want cleaner syntax at call sites
✅ Type inference should work naturally

**Examples:**
- `Iterator` trait uses associated type `Item`
- `Future` trait uses associated type `Output`
- Our `Repository` trait uses associated type `Entity`

### Use Generic Parameters When:

✅ Multiple types should be supported per implementor
✅ The trait might be implemented multiple times with different types
✅ The type is more like a "configuration" than a fundamental property

**Examples:**
- `From<T>` trait - a type can implement `From` for many types
- `Add<Rhs>` trait - addition can work with different right-hand sides
- Collection traits with different element types

## Related Decisions

- [Zero-Sized Repositories](03-zero-sized-repos.md) - Repository structure
- [Entity vs Wrapper Types](05-entity-vs-wrapper.md) - What type goes in `Entity`
- [Executor Passing](01-executor-passing.md) - Why executors aren't associated types

## References

- [Rust Book - Associated Types](https://doc.rust-lang.org/book/ch19-03-advanced-traits.html#specifying-placeholder-types-in-trait-definitions-with-associated-types)
- [Associated Types vs Generic Parameters](https://github.com/rust-lang/rfcs/blob/master/text/0195-associated-items.md)

## See Also

- [Core Traits](../architecture/core-traits.md) - Full `Repository` trait definition
- [PostRepository](../repositories/post-repository.md) - Concrete implementation example
