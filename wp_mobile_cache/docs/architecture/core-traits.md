# Core Traits

> **Last Updated:** 2025-10-22

The core trait system provides database abstraction while maintaining type safety and simplicity.

## Overview

Three key traits form the foundation of the caching architecture:

1. **`QueryExecutor`** - Abstracts database query operations
2. **`TransactionManager`** - Manages database transactions
3. **`DbEntity`** - Marks types as database entities

## QueryExecutor

Abstracts database query execution to decouple from `rusqlite` implementation.

### Definition

```rust
pub trait QueryExecutor {
    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement<'_>, SqliteDbError>;
    fn execute(&self, sql: &str, params: impl rusqlite::Params) -> Result<usize, SqliteDbError>;
    fn last_insert_rowid(&self) -> RowId;
}
```

### Implementations

- **`rusqlite::Connection`** - Direct database access
- **`rusqlite::Transaction`** - Within active transactions

### Usage

```rust
fn select_by_id(
    executor: &impl QueryExecutor,
    site: &DbSite,
    id: RowId,
) -> Result<DbPost, SqliteDbError> {
    let sql = "SELECT * FROM posts WHERE db_site_id = ? AND rowid = ?";
    let mut stmt = executor.prepare(sql)?;
    let post = stmt.query_row([site.row_id, id], |row| /* ... */)?;
    Ok(post)
}
```

### Design Trade-offs

- **Still references `rusqlite` types** - `Statement`, `Params` types remain for simplicity
- **Explicit lifetimes** - Uses `'_` for clippy compliance
- **Minimal abstraction** - Can be extended for other databases if needed

See [Design Decision 4: Minimal Abstraction](../design-decisions/04-minimal-abstraction.md) for rationale.

## TransactionManager

Manages database transactions while preventing nesting at compile-time.

### Definition

```rust
pub trait TransactionManager: QueryExecutor {
    fn transaction(&mut self) -> Result<rusqlite::Transaction<'_>, SqliteDbError>;
}
```

### Implementation

- **`rusqlite::Connection`** implements `TransactionManager` + `QueryExecutor`
- **`rusqlite::Transaction`** implements only `QueryExecutor`

### Type-Level Safety

This design prevents nested transactions at compile-time:

```rust
// ✅ Connection can create transactions
let tx = conn.transaction()?;

// ❌ Transaction cannot - compiler error
let nested = tx.transaction()?; // Won't compile!
```

See [Design Decision 8: Split Traits](../design-decisions/08-split-traits.md) for rationale.

## DbEntity

Marks types that represent database entities.

### Definition

```rust
pub trait DbEntity {
    const TABLE_NAME: &'static str;
}
```

### Example

```rust
impl DbEntity for AnyPostWithEditContext {
    const TABLE_NAME: &'static str = "posts_edit_context";
}
```

See [Design Decision 5: Entity vs Wrapper Types](../design-decisions/05-entity-vs-wrapper.md) for rationale.

## Design Rationale

For detailed explanations of architectural decisions:

- [Why Pass Executor Explicitly?](../design-decisions/01-executor-passing.md)
- [Why Associated Types?](../design-decisions/02-associated-types.md)
- [Why Zero-Sized Repositories?](../design-decisions/03-zero-sized-repos.md)

## See Also

- [Type System](type-system.md) - `RowId`, `DbSite`, wrapper types
- [Database Schema](database-schema.md) - Table definitions
- [Usage Examples](../usage-examples.md) - Common patterns
