# Design Decision 8: Split QueryExecutor and TransactionManager Traits

> **Last Updated:** 2025-10-21

## Decision

Separate transaction management from query execution with two distinct traits.

## Context

Database access involves two operations:
1. Executing queries (SELECT, INSERT, UPDATE, DELETE)
2. Managing transactions (BEGIN, COMMIT, ROLLBACK)

We need to decide how to model these in our trait system.

## Rationale

### Type-Level Safety

**Prevents nested transactions at compile time:**

```rust
// ✅ Connection can create transactions
let mut conn = Connection::open("cache.db")?;
let tx = conn.transaction()?;  // Compiles

// ❌ Transaction cannot create nested transactions
let tx = conn.transaction()?;
let nested = tx.transaction()?;  // Won't compile!
// Error: Transaction doesn't implement TransactionManager
```

**SQLite doesn't support nested transactions:**
- Attempting nested transactions causes runtime errors
- Type system catches this at compile time
- No runtime checks needed

### Clear Separation of Concerns

**Query execution vs transaction management are different responsibilities:**

```rust
// QueryExecutor - Core database operations
pub trait QueryExecutor {
    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement<'_>>;
    fn execute(&self, sql: &str, params: impl rusqlite::Params) -> Result<usize>;
    fn last_insert_rowid(&self) -> RowId;
}

// TransactionManager - Transaction lifecycle
pub trait TransactionManager: QueryExecutor {
    fn transaction(&mut self) -> Result<rusqlite::Transaction<'_>>;
}
```

**Benefits:**
- ✅ Single responsibility per trait
- ✅ Clear intent in method signatures
- ✅ Can implement one without the other

### Flexible Usage

**Repository methods can require appropriate trait:**

```rust
impl PostRepository {
    // Read-only operations - only need QueryExecutor
    pub fn select_by_rowid(
        &self,
        executor: &impl QueryExecutor,  // Accept Connection OR Transaction
        site: &DbSite,
        rowid: RowId,
    ) -> Result<DbAnyPostWithEditContext> {
        // Can use Connection or Transaction
    }

    // Batch operations - need TransactionManager
    pub fn insert_batch(
        &self,
        transaction_manager: &mut impl TransactionManager,  // Only Connection
        items: &[AnyPostWithEditContext],
        site: &DbSite,
    ) -> Result<Vec<RowId>> {
        let tx = transaction_manager.transaction()?;
        // Use transaction for atomicity
    }
}
```

**Intent is explicit:**
- Methods accepting `QueryExecutor` - work with or without transactions
- Methods accepting `TransactionManager` - will create a transaction

### No Runtime Overhead

**Compiler enforces the rules, no runtime checks:**

```rust
// No need for:
if let Some(tx_manager) = executor.as_transaction_manager() {
    // ...
}

// Compiler ensures correct type at compile time
```

**Zero-cost abstraction:**
- No dynamic dispatch needed
- No runtime type checking
- Pure compile-time safety

## Implementation

### Trait Definitions

```rust
/// Core database query operations
pub trait QueryExecutor {
    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement<'_>, SqliteDbError>;
    fn execute(&self, sql: &str, params: impl rusqlite::Params) -> Result<usize, SqliteDbError>;
    fn last_insert_rowid(&self) -> RowId;
}

/// Transaction management (requires QueryExecutor)
pub trait TransactionManager: QueryExecutor {
    fn transaction(&mut self) -> Result<rusqlite::Transaction<'_>, SqliteDbError>;
}
```

### Implementations

```rust
// Connection implements BOTH traits
impl QueryExecutor for rusqlite::Connection {
    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement<'_>, SqliteDbError> {
        self.prepare(sql).map_err(SqliteDbError::from)
    }

    fn execute(&self, sql: &str, params: impl rusqlite::Params) -> Result<usize, SqliteDbError> {
        self.execute(sql, params).map_err(SqliteDbError::from)
    }

    fn last_insert_rowid(&self) -> RowId {
        RowId(self.last_insert_rowid())
    }
}

impl TransactionManager for rusqlite::Connection {
    fn transaction(&mut self) -> Result<rusqlite::Transaction<'_>, SqliteDbError> {
        self.transaction().map_err(SqliteDbError::from)
    }
}

// Transaction implements ONLY QueryExecutor
impl QueryExecutor for rusqlite::Transaction<'_> {
    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement<'_>, SqliteDbError> {
        self.prepare(sql).map_err(SqliteDbError::from)
    }

    fn execute(&self, sql: &str, params: impl rusqlite::Params) -> Result<usize, SqliteDbError> {
        self.execute(sql, params).map_err(SqliteDbError::from)
    }

    fn last_insert_rowid(&self) -> RowId {
        RowId(self.last_insert_rowid())
    }
}

// Note: Transaction does NOT implement TransactionManager
```

## Example Usage

### Read Operations (QueryExecutor)

```rust
// Works with Connection
let conn = Connection::open("cache.db")?;
let post = repo.select_by_rowid(&conn, &site, rowid)?;

// Also works with Transaction
let tx = conn.transaction()?;
let post = repo.select_by_rowid(&tx, &site, rowid)?;
```

### Write Operations in Transaction

```rust
let mut conn = Connection::open("cache.db")?;
let tx = conn.transaction()?;

// Use transaction for multiple operations
repo.insert(&tx, &post1, &site)?;
repo.insert(&tx, &post2, &site)?;

tx.commit()?;
```

### Batch Operations (TransactionManager)

```rust
impl Repository for PostRepository {
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

// Called with Connection
let mut conn = Connection::open("cache.db")?;
let rowids = repo.insert_batch(&mut conn, &posts, &site)?;
```

### Preventing Nested Transactions

```rust
fn process_in_transaction<T: TransactionManager>(tx_manager: &mut T) -> Result<()> {
    let tx = tx_manager.transaction()?;
    // ... work with tx
    Ok(())
}

// ✅ Works with Connection
let mut conn = Connection::open("cache.db")?;
process_in_transaction(&mut conn)?;

// ❌ Won't compile with Transaction
let tx = conn.transaction()?;
process_in_transaction(&mut tx)?;  // Compile error!
// Error: Transaction doesn't implement TransactionManager
```

## Alternatives Considered

### Alternative 1: Single Trait with Optional Transaction

```rust
pub trait DatabaseExecutor {
    fn prepare(&self, sql: &str) -> Result<Statement>;
    fn execute(&self, sql: &str, params: Params) -> Result<usize>;
    fn transaction(&mut self) -> Option<Transaction>;  // Returns None for Transaction
}

// Usage
if let Some(tx) = executor.transaction() {
    // Has transaction support
} else {
    // No transaction support
}
```

**Why rejected:**
- ❌ Runtime check instead of compile-time
- ❌ Can forget to check None case
- ❌ Less type-safe
- ❌ Unclear intent from signature

### Alternative 2: Separate Traits, No Inheritance

```rust
pub trait QueryExecutor {
    fn prepare(&self, sql: &str) -> Result<Statement>;
}

pub trait TransactionManager {
    fn transaction(&mut self) -> Result<Transaction>;
}

// No relationship between traits
```

**Why rejected:**
- ❌ Can't use transaction result as QueryExecutor
- ❌ Need to accept both traits separately
- ❌ More verbose function signatures
- ❌ Loses the "TransactionManager can also execute queries" relationship

### Alternative 3: Concrete Types Only

```rust
// No traits, just use Connection and Transaction directly
impl PostRepository {
    pub fn select_by_rowid(&self, conn: &Connection, ...) -> Result<Post> { }
    pub fn select_by_rowid_in_tx(&self, tx: &Transaction, ...) -> Result<Post> { }
}
```

**Why rejected:**
- ❌ Code duplication (need two versions of each method)
- ❌ Cannot write generic code
- ❌ Harder to test (can't mock)
- ❌ Less flexible

### Alternative 4: All Methods Take Connection

```rust
impl PostRepository {
    // Always take connection, create transaction internally if needed
    pub fn insert_batch(&self, conn: &mut Connection, ...) -> Result<Vec<RowId>> {
        let tx = conn.transaction()?;
        // ...
    }
}
```

**Why rejected:**
- ❌ Caller cannot control transaction scope
- ❌ Cannot compose multiple operations in one transaction
- ❌ Less flexible

## Trade-offs

### Advantages

✅ **Compile-time safety** - Prevents nested transactions at compile time
✅ **Clear intent** - Method signature shows if transactions needed
✅ **Zero runtime overhead** - Compiler enforces rules
✅ **Flexible** - Methods work with Connection or Transaction as appropriate
✅ **Type-safe** - Cannot accidentally misuse

### Disadvantages

❌ **Slightly more complex** - Two traits instead of one
❌ **More verbose** - Need to specify correct trait bound
❌ **Requires understanding** - Developers must understand the split

**Mitigation:**
- Complexity is minimal (just two traits)
- Verbosity is offset by safety
- Good documentation explains the pattern
- Benefits far outweigh costs

## Design Pattern

This pattern is called **capability-based security** or **type-level capability**:

- `QueryExecutor` = capability to execute queries
- `TransactionManager` = capability to create transactions

Types that have both capabilities implement both traits.
Types that have only query capability implement only `QueryExecutor`.

The type system enforces these capabilities at compile time.

## Related Decisions

- [Core Traits](../architecture/core-traits.md) - Full trait definitions
- [Executor Passing](01-executor-passing.md) - Why executors are parameters
- [Minimal Abstraction](04-minimal-abstraction.md) - Abstraction strategy

## References

- [Capability-Based Security](https://en.wikipedia.org/wiki/Capability-based_security)
- [Type-Level Programming](https://willcrichton.net/notes/type-level-programming/)

## See Also

- [Usage Examples](../usage-examples.md) - Transaction usage patterns
- [PostRepository](../repositories/post-repository.md) - Methods using both traits
