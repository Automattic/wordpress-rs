# Repository Pattern Design for wp_mobile_cache

## Overview

This document outlines the design for a Repository pattern to abstract database operations in the `wp_mobile_cache` crate. The pattern provides a consistent interface for CRUD operations while allowing type-specific customization with multi-site support.

## Goals

1. **Reduce boilerplate**: Eliminate repetitive database query code across types
2. **Type safety**: Maintain strong typing for database entities
3. **Flexibility**: Allow custom methods per entity type (e.g., `select_by_post_id` for posts)
4. **Database abstraction**: Decouple from `rusqlite` implementation details
5. **Simplicity**: Keep the design simple and explicit, avoiding over-engineering
6. **Multi-site support**: All operations are scoped to a specific site via `site_id`

## Non-Goals

1. **Complex wrappers**: We will NOT create complex wrapper types that hold connections
2. **ORM features**: This is not a full ORM - no lazy loading, change tracking, etc.
3. **Query builders**: Complex query building DSLs are out of scope

## Architecture

### Core Components

#### 1. QueryExecutor and TransactionManager Traits

Abstracts database query execution to decouple from `rusqlite`.

**QueryExecutor** - Core query operations:
```rust
pub trait QueryExecutor {
    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement<'_>, SqliteDbError>;
    fn execute(&self, sql: &str, params: impl rusqlite::Params) -> Result<usize, SqliteDbError>;
    fn last_insert_rowid(&self) -> RowId;
}
```

**TransactionManager** - Transaction management:
```rust
pub trait TransactionManager: QueryExecutor {
    fn transaction(&mut self) -> Result<rusqlite::Transaction<'_>, SqliteDbError>;
}
```

**Implementation:**
- `rusqlite::Connection` implements both `QueryExecutor` and `TransactionManager`
- `rusqlite::Transaction` implements only `QueryExecutor` (cannot create nested transactions)
- This design prevents nested transactions at the type level (compile-time safety)

**Trade-offs:**
- Still references `rusqlite` types (Statement, Params) to avoid over-abstraction
- Explicit `'_` lifetimes added for clippy compliance
- Can be further abstracted if we need to support other databases

#### 2. DbEntity Trait

Marks types that can be persisted to the database.

```rust
pub trait DbEntity: InsertIntoDb {
    const TABLE_NAME: &'static str;
}
```

**Requirements:**
- Types must implement `InsertIntoDb` (serialization to database)
- Types must specify their table name

**Example:**
```rust
impl DbEntity for AnyPostWithEditContext {
    const TABLE_NAME: &'static str = "posts_edit_context";
}
```

**Note:** `TryFromDbRow` is implemented on wrapper types (e.g., `DbAnyPostWithEditContext`) that include the database rowid, not on the domain entity itself.

#### 3. Repository Trait

Provides common operations with default implementations.

```rust
pub trait Repository {
    type Entity: DbEntity;

    fn insert(&self, executor: &impl QueryExecutor, item: &Self::Entity, site: &DbSite)
        -> Result<RowId, SqliteDbError> {
        item.insert_into_db(executor, site)
    }

    fn insert_batch(&self, transaction_manager: &mut impl TransactionManager,
                    items: &[Self::Entity], site: &DbSite)
        -> Result<Vec<RowId>, SqliteDbError> {
        // Default implementation uses a transaction
    }
}
```

**Default Implementations:**
- `insert`: Delegates to `InsertIntoDb::insert_into_db`, accepts any `QueryExecutor`
- `insert_batch`: Uses `TransactionManager` to wrap inserts in a transaction for atomicity

**Multi-Site Design:**
- All insert operations require a `site` parameter to scope data to a specific site
- The `site.row_id` is passed through to the database layer where it's stored with the entity

**Note:** Query methods (`select_by_rowid`, `select_all`, etc.) are NOT part of the trait because they need to return wrapper types (e.g., `DbAnyPostWithEditContext`) that include the database rowid and site. Concrete repositories implement these directly.

#### 4. Concrete Repositories

Type-specific repositories with custom methods.

```rust
pub struct PostRepository;

impl Repository for PostRepository {
    type Entity = AnyPostWithEditContext;
}

impl PostRepository {
    // Query methods return DbAnyPostWithEditContext (includes rowid and site_id)
    pub fn select_by_rowid(&self, executor: &impl QueryExecutor, site: &DbSite, rowid: RowId)
        -> Result<DbAnyPostWithEditContext, SqliteDbError> { /* ... */ }

    pub fn select_all(&self, executor: &impl QueryExecutor, site: &DbSite)
        -> Result<Vec<DbAnyPostWithEditContext>, SqliteDbError> { /* ... */ }

    pub fn select_by_post_id(&self, executor: &impl QueryExecutor, site: &DbSite, post_id: PostId)
        -> Result<DbAnyPostWithEditContext, SqliteDbError> { /* ... */ }

    pub fn select_by_author(&self, executor: &impl QueryExecutor, site: &DbSite, author_id: UserId)
        -> Result<Vec<DbAnyPostWithEditContext>, SqliteDbError> { /* ... */ }

    pub fn select_by_status(&self, executor: &impl QueryExecutor, site: &DbSite, status: &str)
        -> Result<Vec<DbAnyPostWithEditContext>, SqliteDbError> { /* ... */ }

    // Upsert (insert or update) using SQLite's ON CONFLICT
    pub fn upsert(&self, executor: &impl QueryExecutor, site: &DbSite, post: &AnyPostWithEditContext)
        -> Result<RowId, SqliteDbError> { /* ... */ }

    // Delete by WordPress post ID and site
    pub fn delete_by_post_id(&self, executor: &impl QueryExecutor, site: &DbSite, post_id: PostId)
        -> Result<usize, SqliteDbError> { /* ... */ }

    // Count total posts for a site
    pub fn count(&self, executor: &impl QueryExecutor, site: &DbSite)
        -> Result<i64, SqliteDbError> { /* ... */ }
}
```

## Design Decisions

### Decision 1: Pass Executor Explicitly

**Decision:** Pass `executor` as a parameter to each method rather than storing it in the repository.

**Rationale:**
- **Simplicity**: No lifetime complexity, no borrow checker issues
- **Flexibility**: Clients can use different executors (Connection, Transaction) per call
- **YAGNI**: Clients can build convenience wrappers if they want them
- **Least committal**: Easy to add wrappers later, hard to remove complexity

**Example:**
```rust
let repo = PostRepository;
let site = DbSite { row_id: RowId(1) };
let post = repo.select_by_rowid(&conn, &site, RowId(42))?;
let post = repo.select_by_post_id(&conn, &site, PostId(123))?;
```

**Client Convenience (Optional):**
If clients want convenience, they can create their own wrappers:
```rust
struct PostRepositoryWithExecutor<E> {
    executor: E,
    repo: PostRepository,
    site: DbSite,
}

impl<E: QueryExecutor> PostRepositoryWithExecutor<E> {
    pub fn select_by_rowid(&self, rowid: RowId) -> Result<DbAnyPostWithEditContext, SqliteDbError> {
        self.repo.select_by_rowid(&self.executor, &self.site, rowid)
    }
}
```

### Decision 2: Use Associated Type (Not Generic Parameter)

**Decision:** Use `type Entity = ...` instead of `Repository<T>`

**Rationale:**
- **1-to-1 relationship**: Each repository is for exactly one entity type
- **Cleaner syntax**: `PostRepository` instead of `Repository<AnyPostWithEditContext>`
- **Better ergonomics**: Simpler to use and implement

**Example:**
```rust
// With associated type (chosen)
impl Repository for PostRepository {
    type Entity = AnyPostWithEditContext;
}

// With generic parameter (rejected)
impl Repository<AnyPostWithEditContext> for PostRepository { }
```

**Note:** The entity type is the domain model (`AnyPostWithEditContext`), not the database wrapper (`DbAnyPostWithEditContext`). Query methods return the wrapper type which includes the rowid.

### Decision 3: Zero-Sized Repositories

**Decision:** Repositories are zero-sized structs with no fields.

**Rationale:**
- **Zero-cost abstraction**: No runtime overhead
- **Stateless**: All operations are stateless, no need for state
- **Singleton-friendly**: Clients can create singletons if they want, but not required

**Example:**
```rust
pub struct PostRepository; // Zero-sized

// Usage (no new() needed)
let repo = PostRepository;
```

### Decision 4: Minimal QueryExecutor Abstraction

**Decision:** Abstract over query execution, but still use `rusqlite` types in the trait.

**Rationale:**
- **Pragmatic**: We're using `rusqlite` everywhere else anyway
- **Avoid over-engineering**: Full database abstraction is overkill for now
- **Future-proof**: Can be extended if needed

**Trade-off:**
- Not fully database-agnostic, but that's okay for now

### Decision 5: Entity vs Wrapper Types

**Decision:** Separate domain entities from database wrappers.

**Rationale:**
- **Domain model**: `AnyPostWithEditContext` represents the WordPress post (from the API)
- **Database wrapper**: `DbAnyPostWithEditContext` includes the SQLite rowid plus the post
- **Repository entity type**: Uses the domain model for `insert` operations
- **Query return type**: Returns the wrapper to provide access to rowid when needed

**Example:**
```rust
// Domain entity (no rowid or site_id)
pub struct AnyPostWithEditContext {
    pub id: PostId,  // WordPress post ID
    pub title: PostTitleWithEditContext,
    // ... other WordPress fields
}

// Database wrapper (includes rowid and site_id)
pub struct DbAnyPostWithEditContext {
    pub row_id: RowId,  // SQLite rowid
    pub site: DbSite,  // Site identifier
    pub post: AnyPostWithEditContext,
}
```

### Decision 6: UPSERT for Insert/Update Operations

**Decision:** Use SQLite's `INSERT ... ON CONFLICT ... DO UPDATE` for atomic insert/update.

**Rationale:**
- **Database observers**: Ensures observers see a single INSERT or UPDATE action, not DELETE + INSERT
- **DRY principle**: Write SQL field list only once (shared between INSERT and UPDATE)
- **Rowid preservation**: Updates keep the same rowid (important for consistency)
- **Natural key**: Uses composite key `(db_site_id, id)` for conflict detection (unique index)

**Implementation:**
```rust
pub fn upsert(&self, executor: &impl QueryExecutor, site: &DbSite, post: &AnyPostWithEditContext)
    -> Result<RowId, SqliteDbError> {
    executor.execute(
        r#"
        INSERT INTO posts_edit_context (db_site_id, id, date, ...)
        VALUES (:db_site_id, :id, :date, ...)
        ON CONFLICT(db_site_id, id) DO UPDATE SET
            date = excluded.date,
            ...
        "#,
        named_params! {
            ":db_site_id": site.row_id,
            ":id": post.id.0,
            ...
        }
    )?;
    Ok(QueryExecutor::last_insert_rowid(executor))
}
```

### Decision 7: Multi-Site Architecture with DbSite

**Decision:** All entities are scoped to a site via `DbSite` parameter with foreign key constraints.

**Rationale:**
- **Data isolation**: Posts from different sites cannot conflict (same WordPress post ID allowed per site)
- **Referential integrity**: Foreign key ensures posts cannot exist without a valid site
- **Query scoping**: All queries automatically filter by site, preventing cross-site data leaks
- **Cascade deletion**: Deleting a site automatically removes all associated posts
- **Type safety**: `DbSite` prevents confusion with WordPress.com site IDs or domain identifiers

**Why `DbSite` Instead of a Simple ID?**

Repository methods take `&DbSite` rather than just a numeric ID for several critical reasons:

1. **Prevents confusion with WordPress identifiers**:
   - `DbSite` is clearly a database-internal type (hence the `Db` prefix)
   - Cannot be confused with WordPress.com site IDs
   - Self-hosted sites don't have numeric IDs, making a generic "site ID" misleading

2. **Forces valid site references**:
   - Callers must fetch a valid `DbSite` from a site repository first
   - Prevents arbitrary ID construction like `DbSite { row_id: RowId(999) }`
   - Ensures the site exists in the cache before querying for its posts

3. **Future-proof for site polymorphism**:
   - When site type tables are added (`self_hosted_sites`, `wordpress_com_sites`), `DbSite` will gain fields:
   ```rust
   pub struct DbSite {
       pub row_id: RowId,
       pub site_type: SiteType,      // SelfHosted | WordPressCom
       pub mapped_site_id: RowId,    // FK to specific site type table
   }
   ```
   - These fields enable joins without changing repository APIs

4. **Zero-cost abstraction**:
   - `DbSite` is `Copy` (all primitives), so `&DbSite` has no runtime overhead

**Database Schema:**
```sql
-- Sites table (foundation)
CREATE TABLE `sites` (
  `id` INTEGER PRIMARY KEY AUTOINCREMENT
) STRICT;

-- Posts table with foreign key to sites
CREATE TABLE `posts_edit_context` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `db_site_id` INTEGER NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
  `id` INTEGER NOT NULL,  -- WordPress post ID
  -- ... other fields
  FOREIGN KEY (db_site_id) REFERENCES sites(id) ON DELETE CASCADE
) STRICT;

-- Composite unique index on (db_site_id, id)
CREATE UNIQUE INDEX idx_posts_edit_context_unique_db_site_id_and_id
  ON posts_edit_context(db_site_id, id);

-- Index on db_site_id for query performance
CREATE INDEX idx_posts_edit_context_db_site_id
  ON posts_edit_context(db_site_id);
```

### Decision 8: Split QueryExecutor and TransactionManager Traits

**Decision:** Separate transaction management from query execution with two distinct traits.

**Rationale:**
- **Type-level safety**: Prevents nested transactions at compile time
- **Clear separation of concerns**: Query execution vs transaction management
- **Flexible usage**: Repository methods can accept `QueryExecutor` for read operations, or `TransactionManager` when transactions are needed
- **No runtime overhead**: Compiler enforces the rules, no panics or runtime checks

**Implementation:**
- `QueryExecutor`: Core trait with `prepare()`, `execute()`, `last_insert_rowid()`
- `TransactionManager`: Extends `QueryExecutor`, adds `transaction()` method
- `Connection` implements both traits (can execute queries and create transactions)
- `Transaction` implements only `QueryExecutor` (can execute queries but cannot create nested transactions)

**Example:**
```rust
// Repository methods that only need to execute queries
pub fn select_by_rowid(&self, executor: &impl QueryExecutor, ...) { }

// Repository methods that need transaction support
pub fn insert_batch(&self, transaction_manager: &mut impl TransactionManager, ...) {
    let tx = transaction_manager.transaction()?;
    // ... work with tx (which implements QueryExecutor)
}
```

**Trade-offs:**
- Slightly more complex trait hierarchy, but gains compile-time safety
- Makes the API intent clearer: methods requiring transactions are explicit about it

## Implementation Plan

### Phase 1: Core Traits
1. Create `QueryExecutor` trait
2. Implement `QueryExecutor` for `rusqlite::Connection`
3. Create `DbEntity` trait
4. Implement `DbEntity` for `DbAnyPostWithEditContext`

### Phase 2: Repository Trait
1. Create `Repository` trait
2. Implement default methods for common operations
3. Add transaction support to `insert_batch`

### Phase 3: Concrete Repository
1. Create `PostRepository` struct
2. Implement `Repository` for `PostRepository`
3. Add custom methods (`select_by_post_id`, etc.)

### Phase 4: Testing
1. Update existing tests to use Repository pattern
2. Add tests for custom methods
3. Add tests for batch operations
4. Add tests for upsert (insert and update scenarios)

## Usage Examples

### Basic Usage

```rust
use wp_mobile_cache::repository::{Repository, PostRepository};
use wp_mobile_cache::{DbSite, RowId};

let conn = Connection::open("cache.db")?;
let repo = PostRepository;
let site = DbSite { row_id: RowId(1) };

// Insert operation (from trait, returns RowId)
let post = create_post();
let rowid: RowId = repo.insert(&conn, &post, &site)?;

// Query operations (return DbAnyPostWithEditContext with rowid and site)
let db_post = repo.select_by_rowid(&conn, &site, RowId(42))?;
let all_posts = repo.select_all(&conn, &site)?;

// Custom query operations
let db_post = repo.select_by_post_id(&conn, &site, PostId(123))?;
let author_posts = repo.select_by_author(&conn, &site, UserId(1))?;
let draft_posts = repo.select_by_status(&conn, &site, "draft")?;

// Upsert (insert or update, returns RowId)
let rowid: RowId = repo.upsert(&conn, &site, &updated_post)?;

// Delete
let deleted_count = repo.delete_by_post_id(&conn, &site, PostId(123))?;

// Count
let total = repo.count(&conn, &site)?;
```

### Batch Operations

```rust
let posts = vec![post1, post2, post3];
let mut conn = Connection::open("cache.db")?;
let site = DbSite { row_id: RowId(1) };
let rowids: Vec<RowId> = repo.insert_batch(&mut conn, &posts, &site)?;
```

**Note:** `insert_batch` requires a mutable `TransactionManager` (Connection) because it uses a transaction internally. All posts in the batch are inserted for the same site.

## Future Enhancements

### Possible Additions

1. **Query builder**: For complex WHERE clauses with multiple conditions
2. **Pagination**: `select_page(&self, executor: &impl QueryExecutor, offset: usize, limit: usize)`
3. **Ordering**: `select_all_ordered(&self, executor: &impl QueryExecutor, order_by: &str)`
4. **Bulk operations**: Optimized bulk upsert for syncing large datasets
5. **Soft deletes**: Mark records as deleted without removing them

### Database Abstraction Evolution

If we need to support other databases:
1. Create `RowReader` trait to abstract `rusqlite::Row`
2. Create `Params` trait to abstract parameter binding
3. Create `Statement` trait to abstract prepared statements

## File Organization

```
wp_mobile_cache/
├── migrations/
│   ├── 0001-create-sites-table.sql     # Sites table (foundation)
│   ├── 0002-create-posts-table.sql     # Posts table with FK to sites
│   └── ...                              # Future migrations
├── src/
│   ├── repository/
│   │   ├── mod.rs           # QueryExecutor, DbEntity, Repository traits
│   │   ├── posts.rs         # PostRepository
│   │   └── ...              # Future repositories
│   ├── mappings/
│   │   ├── posts.rs         # DbEntity implementations
│   │   └── ...
│   └── lib.rs               # DbSite, migration management
└── REPOSITORY_PATTERN_DESIGN.md
```

## Conclusion

This design provides a clean, simple abstraction for database operations while maintaining flexibility for type-specific customization. The explicit executor passing keeps the design simple and allows clients to add their own convenience layers as needed.

The multi-site architecture ensures data isolation through foreign key constraints and composite unique indexes, allowing the same WordPress post IDs across different sites without conflicts. All repository methods require a `site_id` parameter, enforcing proper data scoping at the API level.
