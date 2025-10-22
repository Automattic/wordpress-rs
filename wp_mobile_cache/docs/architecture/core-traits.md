# Core Traits

> **Last Updated:** 2025-10-21

The core trait system provides database abstraction and repository operations while maintaining type safety and simplicity.

## Overview

Four key traits form the foundation of the caching architecture:

1. **`QueryExecutor`** - Abstracts database query operations
2. **`TransactionManager`** - Manages database transactions
3. **`DbEntity`** - Marks types as database entities
4. **`Repository`** - Provides common CRUD operations

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

### Usage

```rust
pub fn insert_batch(
    &self,
    transaction_manager: &mut impl TransactionManager,
    items: &[Post],
    site: &DbSite,
) -> Result<Vec<RowId>, SqliteDbError> {
    let tx = transaction_manager.transaction()?;
    let mut rowids = Vec::new();

    for item in items {
        rowids.push(self.insert(&tx, item, site)?);
    }

    tx.commit()?;
    Ok(rowids)
}
```

See [Design Decision 8: Split Traits](../design-decisions/08-split-traits.md) for rationale.

## DbEntity

Marks types that can be persisted to the database.

### Definition

```rust
pub trait DbEntity: InsertIntoDb {
    const TABLE_NAME: &'static str;
}
```

### Requirements

- Must implement `InsertIntoDb` for serialization
- Must specify table name via constant

### Example

```rust
impl DbEntity for AnyPostWithEditContext {
    const TABLE_NAME: &'static str = "posts_edit_context";
}
```

### Important Note

`TryFromDbRow` is implemented on **wrapper types** (e.g., `DbAnyPostWithEditContext`) that include database metadata, not on the domain entity itself.

```rust
// Wrapper type with DB metadata
pub struct DbAnyPostWithEditContext {
    pub row_id: RowId,
    pub site: DbSite,
    pub post: AnyPostWithEditContext,  // Domain entity
    pub last_fetched_at: String,
}

impl TryFromDbRow for DbAnyPostWithEditContext {
    // Deserialize from SQL row
}
```

See [Design Decision 5: Entity vs Wrapper Types](../design-decisions/05-entity-vs-wrapper.md) for rationale.

## Repository

Provides common CRUD operations with default implementations.

### Definition

```rust
pub trait Repository {
    type Entity: DbEntity;

    fn insert(
        &self,
        executor: &impl QueryExecutor,
        item: &Self::Entity,
        site: &DbSite,
    ) -> Result<RowId, SqliteDbError> {
        item.insert_into_db(executor, site)
    }

    fn insert_batch(
        &self,
        transaction_manager: &mut impl TransactionManager,
        items: &[Self::Entity],
        site: &DbSite,
    ) -> Result<Vec<RowId>, SqliteDbError> {
        let tx = transaction_manager.transaction()?;
        let mut rowids = Vec::new();
        for item in items {
            rowids.push(self.insert(&tx, item, site)?);
        }
        tx.commit()?;
        Ok(rowids)
    }
}
```

### Default Implementations

- **`insert`** - Delegates to `InsertIntoDb::insert_into_db`
- **`insert_batch`** - Wraps multiple inserts in a transaction

### Multi-Site Design

All operations require `&DbSite` parameter to scope data:

```rust
let site = DbSite { row_id: RowId(1) };
repo.insert(&conn, &post, &site)?;
```

### Query Methods Not Included

Query methods (`select_by_rowid`, `select_all`, etc.) are **not** part of the trait because:

- They need to return wrapper types (e.g., `DbAnyPostWithEditContext`)
- Each entity has different query needs
- Concrete repositories implement these directly

### Concrete Repository Example

```rust
pub struct PostRepository;

impl Repository for PostRepository {
    type Entity = AnyPostWithEditContext;
}

impl PostRepository {
    // Custom query methods
    pub fn select_by_rowid(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        rowid: RowId,
    ) -> Result<DbAnyPostWithEditContext, SqliteDbError> {
        // Implementation...
    }

    pub fn select_by_post_id(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        post_id: PostId,
    ) -> Result<DbAnyPostWithEditContext, SqliteDbError> {
        // Implementation...
    }

    pub fn upsert(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        post: &AnyPostWithEditContext,
    ) -> Result<RowId, SqliteDbError> {
        // SQLite UPSERT implementation...
    }
}
```

See [PostRepository API](../repositories/post-repository.md) for complete documentation.

## Design Rationale

For detailed explanations of architectural decisions:

- [Why Pass Executor Explicitly?](../design-decisions/01-executor-passing.md)
- [Why Associated Types?](../design-decisions/02-associated-types.md)
- [Why Zero-Sized Repositories?](../design-decisions/03-zero-sized-repos.md)

## See Also

- [Type System](type-system.md) - `RowId`, `DbSite`, wrapper types
- [Database Schema](database-schema.md) - Table definitions
- [Usage Examples](../usage-examples.md) - Common patterns
