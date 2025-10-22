# Design Decision 4: Minimal QueryExecutor Abstraction

> **Last Updated:** 2025-10-21

## Decision

Abstract over query execution, but still use `rusqlite` types in the trait.

## Context

Database abstraction exists on a spectrum:

```rust
// Option 1: Full abstraction (rejected)
trait QueryExecutor {
    type Statement;
    type Params;
    type Row;
    fn prepare(&self, sql: &str) -> Result<Self::Statement>;
    fn execute(&self, sql: &str, params: Self::Params) -> Result<usize>;
}

// Option 2: Minimal abstraction (chosen)
trait QueryExecutor {
    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement<'_>>;
    fn execute(&self, sql: &str, params: impl rusqlite::Params) -> Result<usize>;
    fn last_insert_rowid(&self) -> RowId;
}

// Option 3: No abstraction (rejected)
// Just use rusqlite::Connection directly everywhere
```

## Rationale

### Pragmatic Design

**We're using `rusqlite` everywhere anyway:**

```rust
// Our repository implementations use rusqlite types
pub fn select_by_rowid(
    &self,
    executor: &impl QueryExecutor,
    site: &DbSite,
    rowid: RowId,
) -> Result<DbAnyPostWithEditContext, SqliteDbError> {
    let mut stmt = executor.prepare(
        "SELECT * FROM posts_edit_context WHERE db_site_id = ? AND rowid = ?"
    )?;

    // Returns rusqlite::Statement - we use it directly
    stmt.query_row(
        rusqlite::params![site.row_id.0, rowid.0],
        |row| DbAnyPostWithEditContext::try_from_db_row(row)
    )
}
```

**Current reality:**
- SQLite is the only database we support
- No plans for other databases
- Full abstraction would add complexity without benefit

### Avoid Over-Engineering

**Full database abstraction is overkill:**

What we'd need for full abstraction:
- Abstract `Statement` type
- Abstract `Params` type
- Abstract `Row` type
- Abstract error types
- Custom parameter binding trait
- Custom row reading trait

**Cost:**
- Hundreds of lines of abstraction code
- Runtime overhead (potential trait objects)
- Harder to understand and maintain
- More difficult to debug

**Benefit:**
- Could theoretically support other databases
- But we don't need that now (YAGNI)

### Future-Proof Enough

**The abstraction we have is sufficient:**

Current design allows:
1. ✅ Testing with mock executors
2. ✅ Using Connection or Transaction
3. ✅ Custom executor implementations if needed
4. ✅ Decoupling repository logic from connection management

**If we need more abstraction later:**
- Can add it incrementally
- Won't break existing code
- Start with what we need, not what we might need

### Explicit Lifetimes

**Clippy compliance:**

```rust
// ✅ Explicit lifetime for clarity
fn prepare(&self, sql: &str) -> Result<rusqlite::Statement<'_>>;

// ❌ Elided lifetime (clippy warning)
fn prepare(&self, sql: &str) -> Result<rusqlite::Statement>;
```

The `'_` makes the lifetime relationship explicit.

## Example Usage

### Direct rusqlite Usage

```rust
impl PostRepository {
    pub fn select_by_post_id(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        post_id: PostId,
    ) -> Result<DbAnyPostWithEditContext, SqliteDbError> {
        // Use rusqlite types directly
        let mut stmt = executor.prepare(
            "SELECT * FROM posts_edit_context WHERE db_site_id = ? AND id = ?"
        )?;

        stmt.query_row(
            rusqlite::params![site.row_id.0, post_id.0],
            |row| {
                // rusqlite::Row type used directly
                Ok(DbAnyPostWithEditContext {
                    row_id: RowId(row.get(0)?),
                    site: *site,
                    post: AnyPostWithEditContext {
                        id: PostId(row.get(2)?),
                        // ... map other fields
                    },
                    last_fetched_at: row.get(3)?,
                })
            },
        )
    }
}
```

### Testing with Minimal Abstraction

```rust
// Can still create test doubles
struct MockExecutor {
    should_fail: bool,
}

impl QueryExecutor for MockExecutor {
    fn prepare(&self, _sql: &str) -> Result<rusqlite::Statement<'_>> {
        // Return mock statement or error
        if self.should_fail {
            Err(SqliteDbError::Query("Mock error".into()))
        } else {
            // Can use in-memory database for real statements
            todo!()
        }
    }

    fn execute(&self, _sql: &str, _params: impl rusqlite::Params) -> Result<usize> {
        Ok(1)
    }

    fn last_insert_rowid(&self) -> RowId {
        RowId(42)
    }
}
```

## Alternatives Considered

### Alternative 1: Full Database Abstraction

```rust
pub trait QueryExecutor {
    type Statement: PreparedStatement;
    type Error: Error;

    fn prepare(&self, sql: &str) -> Result<Self::Statement, Self::Error>;
    fn execute(&self, sql: &str, params: &[&dyn ToSql]) -> Result<usize, Self::Error>;
}

pub trait PreparedStatement {
    type Row: RowReader;

    fn query_row<F, T>(&mut self, params: &[&dyn ToSql], f: F) -> Result<T>
    where
        F: FnOnce(&Self::Row) -> Result<T>;
}

pub trait RowReader {
    fn get<T: FromSql>(&self, idx: usize) -> Result<T>;
}
```

**Why rejected:**
- Massive increase in complexity
- No current need for multiple database backends
- Would slow down development
- Harder to debug and maintain
- Runtime overhead from trait objects

### Alternative 2: No Abstraction

```rust
// No QueryExecutor trait - use Connection directly
impl PostRepository {
    pub fn select_by_id(
        &self,
        conn: &Connection,  // Concrete type
        site: &DbSite,
        id: RowId,
    ) -> Result<Post> {
        // ...
    }
}
```

**Why rejected:**
- Cannot use with transactions (Transaction is different type)
- Harder to test (need real database)
- Less flexible
- Cannot abstract "executor" concept

### Alternative 3: Generic Database Library

Use an existing database abstraction like `diesel` or `sqlx`:

**Why rejected:**
- `diesel`: Too opinionated, requires ORM patterns
- `sqlx`: Async only, we want sync
- Both add significant dependencies
- We need minimal, targeted abstraction
- Direct `rusqlite` is simpler for our needs

## Trade-offs

### Advantages

✅ **Simple** - Easy to understand and implement
✅ **Performant** - No abstraction overhead
✅ **Direct** - Use rusqlite features directly
✅ **Practical** - Solves current needs without over-engineering
✅ **Testable** - Can still mock if needed
✅ **Flexible** - Works with Connection and Transaction

### Disadvantages

❌ **rusqlite dependency** - Trait references rusqlite types
❌ **Not database-agnostic** - Cannot easily switch databases
❌ **Less portable** - Tied to SQLite

**Mitigation:**
- These disadvantages are acceptable because we're committed to SQLite
- If we need other databases, we can add abstraction then
- Current design doesn't prevent future abstraction

## When to Abstract Further

Consider more abstraction if:
- Need to support multiple database backends
- Want database-agnostic repository tests
- Migrating away from SQLite
- Multiple team members prefer different databases

**Current status:** None of these apply, so minimal abstraction is appropriate.

## Related Decisions

- [Executor Passing](01-executor-passing.md) - Why executors are passed as parameters
- [Split Traits](08-split-traits.md) - QueryExecutor vs TransactionManager separation
- [Zero-Sized Repositories](03-zero-sized-repos.md) - Repository design

## References

- [YAGNI Principle](https://en.wikipedia.org/wiki/You_aren%27t_gonna_need_it)
- [rusqlite Documentation](https://docs.rs/rusqlite/)

## See Also

- [Core Traits](../architecture/core-traits.md) - Full QueryExecutor definition
- [Database Schema](../architecture/database-schema.md) - SQLite schema
