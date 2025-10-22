# PostRepository API

> **Last Updated:** 2025-10-22

Complete API documentation for the `PostRepository` type, which manages cached WordPress posts.

## Overview

`PostRepository` provides type-safe database operations for WordPress posts with:
- Multi-site scoping
- Atomic UPSERT operations with term relationship management
- Custom query methods
- Transaction support

## Type Definition

```rust
pub struct PostRepository;
```

**Zero-sized struct** - No fields, no construction overhead.

See [Zero-Sized Repositories](../design-decisions/03-zero-sized-repos.md) for rationale.

## Query Operations

Query methods return `DbAnyPostWithEditContext` (wrapper type with database metadata).

### select_by_rowid

Query post by internal database rowid.

```rust
pub fn select_by_rowid(
    &self,
    executor: &impl QueryExecutor,
    site: &DbSite,
    rowid: RowId,
) -> Result<DbAnyPostWithEditContext, SqliteDbError>
```

**Parameters:**
- `executor` - Database connection or transaction
- `site` - Site scope
- `rowid` - Internal database row identifier

**Returns:**
- `DbAnyPostWithEditContext` - Post with database metadata

**Example:**
```rust
let db_post = repo.select_by_rowid(&conn, &site, RowId(42))?;
println!("Post title: {}", db_post.post.title.raw);
println!("Cached at: {}", db_post.last_fetched_at);
```

**Notes:**
- Fast lookup (primary key)
- Includes term relationships (categories, tags) via JOIN
- Returns error if not found

### select_all

Query all posts for a site.

```rust
pub fn select_all(
    &self,
    executor: &impl QueryExecutor,
    site: &DbSite,
) -> Result<Vec<DbAnyPostWithEditContext>, SqliteDbError>
```

**Parameters:**
- `executor` - Database connection or transaction
- `site` - Site scope

**Returns:**
- `Vec<DbAnyPostWithEditContext>` - All posts for the site

**Example:**
```rust
let all_posts = repo.select_all(&conn, &site)?;
println!("Found {} posts", all_posts.len());

for db_post in all_posts {
    println!("- {} (cached: {})", db_post.post.title.raw, db_post.last_fetched_at);
}
```

**Notes:**
- Returns empty vec if no posts
- Includes term relationships for each post
- Ordered by rowid (insertion order)

### select_by_post_id

Query post by WordPress post ID.

```rust
pub fn select_by_post_id(
    &self,
    executor: &impl QueryExecutor,
    site: &DbSite,
    post_id: PostId,
) -> Result<DbAnyPostWithEditContext, SqliteDbError>
```

**Parameters:**
- `executor` - Database connection or transaction
- `site` - Site scope
- `post_id` - WordPress post ID (from REST API)

**Returns:**
- `DbAnyPostWithEditContext` - Post with database metadata

**Example:**
```rust
let db_post = repo.select_by_post_id(&conn, &site, PostId(123))?;
assert_eq!(db_post.post.id, PostId(123));
```

**Notes:**
- Most common query method
- Uses composite index on `(db_site_id, id)` for performance
- Includes term relationships
- Returns error if not found

### select_by_author

Query all posts by a specific author.

```rust
pub fn select_by_author(
    &self,
    executor: &impl QueryExecutor,
    site: &DbSite,
    author_id: UserId,
) -> Result<Vec<DbAnyPostWithEditContext>, SqliteDbError>
```

**Parameters:**
- `executor` - Database connection or transaction
- `site` - Site scope
- `author_id` - WordPress user ID

**Returns:**
- `Vec<DbAnyPostWithEditContext>` - All posts by the author

**Example:**
```rust
let author_posts = repo.select_by_author(&conn, &site, UserId(1))?;
println!("Author has {} posts", author_posts.len());
```

**Notes:**
- Returns empty vec if no posts found
- Includes term relationships
- Useful for author archives

### select_by_status

Query posts by WordPress post status.

```rust
pub fn select_by_status(
    &self,
    executor: &impl QueryExecutor,
    site: &DbSite,
    status: &str,
) -> Result<Vec<DbAnyPostWithEditContext>, SqliteDbError>
```

**Parameters:**
- `executor` - Database connection or transaction
- `site` - Site scope
- `status` - Post status string (`"publish"`, `"draft"`, `"pending"`, etc.)

**Returns:**
- `Vec<DbAnyPostWithEditContext>` - All posts with that status

**Example:**
```rust
let drafts = repo.select_by_status(&conn, &site, "draft")?;
let published = repo.select_by_status(&conn, &site, "publish")?;

println!("Drafts: {}, Published: {}", drafts.len(), published.len());
```

**Notes:**
- Status values: `"publish"`, `"draft"`, `"pending"`, `"private"`, `"future"`, `"trash"`
- Returns empty vec if no matching posts
- Includes term relationships

## Upsert Operations

### upsert

Insert or update a post with its term relationships in a single atomic transaction.

```rust
pub fn upsert(
    &self,
    transaction_manager: &mut impl TransactionManager,
    site: &DbSite,
    post: &AnyPostWithEditContext,
) -> Result<RowId, SqliteDbError>
```

**Parameters:**
- `transaction_manager` - Mutable connection (creates transaction internally)
- `site` - Site scope
- `post` - WordPress post with categories/tags populated

**Returns:**
- `RowId` - Database rowid (same for insert and update)

**Example:**
```rust
let mut post = AnyPostWithEditContext {
    id: PostId(123),
    categories: Some(vec![TermId(1), TermId(2)]),
    tags: Some(vec![TermId(10), TermId(20)]),
    // ...
};

// First call - inserts post and term relationships
let rowid1 = repo.upsert(&mut conn, &site, &post)?;

// Later, update with different terms
post.categories = Some(vec![TermId(1), TermId(3)]);  // Removed 2, added 3
post.tags = Some(vec![TermId(10)]);                   // Removed 20

let rowid2 = repo.upsert(&mut conn, &site, &post)?;
assert_eq!(rowid1, rowid2);  // Same rowid!
```

**Notes:**
- Atomic transaction (post + terms committed together)
- Preserves rowid on update (important for foreign keys)
- Uses composite unique index `(db_site_id, id)` for conflict detection
- `last_fetched_at` updated to current time on both insert and update
- Syncs term relationships (only changes generate database events)
- Database observers see INSERT or UPDATE action (not DELETE + INSERT)

**Term sync behavior:**
- INSERT for new term relationships
- DELETE for removed term relationships
- No operation for unchanged relationships (minimizes database events)
- Uses `TermRelationshipRepository` internally

See [UPSERT Pattern](../design-decisions/06-upsert-pattern.md) and [Term Normalization](../design-decisions/09-term-normalization.md) for rationale.

### upsert_batch

Upsert multiple posts with their term relationships.

```rust
pub fn upsert_batch(
    &self,
    transaction_manager: &mut impl TransactionManager,
    site: &DbSite,
    posts: &[AnyPostWithEditContext],
) -> Result<Vec<RowId>, SqliteDbError>
```

**Parameters:**
- `transaction_manager` - Mutable connection
- `site` - Site scope (all posts upserted for same site)
- `posts` - Slice of WordPress posts with terms

**Returns:**
- `Vec<RowId>` - Database rowids in same order as input

**Example:**
```rust
let posts = vec![post1, post2, post3];
let rowids = repo.upsert_batch(&mut conn, &site, &posts)?;
println!("Upserted {} posts", rowids.len());
```

**Notes:**
- Each post is upserted in its own transaction
- Partial success is allowed (if post 2 fails, post 1 remains)
- Each successful upsert is atomic (post + terms committed together)

## Delete Operations

### delete_by_post_id

Delete post by WordPress post ID.

```rust
pub fn delete_by_post_id(
    &self,
    executor: &impl QueryExecutor,
    site: &DbSite,
    post_id: PostId,
) -> Result<usize, SqliteDbError>
```

**Parameters:**
- `executor` - Database connection or transaction
- `site` - Site scope
- `post_id` - WordPress post ID

**Returns:**
- `usize` - Number of rows deleted (0 or 1)

**Example:**
```rust
let deleted = repo.delete_by_post_id(&conn, &site, PostId(123))?;
if deleted > 0 {
    println!("Post deleted");
} else {
    println!("Post not found");
}
```

**Notes:**
- Deletes post and associated term relationships
- Returns 0 if post doesn't exist (not an error)
- Term relationships deleted first, then post

## Utility Operations

### count

Count total posts for a site.

```rust
pub fn count(
    &self,
    executor: &impl QueryExecutor,
    site: &DbSite,
) -> Result<i64, SqliteDbError>
```

**Parameters:**
- `executor` - Database connection or transaction
- `site` - Site scope

**Returns:**
- `i64` - Total number of posts

**Example:**
```rust
let total = repo.count(&conn, &site)?;
println!("Total posts: {}", total);
```

**Notes:**
- Fast operation (uses COUNT query)
- Returns 0 if no posts

## Multi-Site Usage

All methods require `&DbSite` parameter for site scoping:

```rust
// Site 1
let site1 = site_repo.get_or_create(&conn, &site1_info)?;
let posts1 = repo.select_all(&conn, &site1)?;

// Site 2
let site2 = site_repo.get_or_create(&conn, &site2_info)?;
let posts2 = repo.select_all(&conn, &site2)?;

// Completely isolated - no cross-site data leaks
```

See [Multi-Site with DbSite](../design-decisions/07-multi-site-dbsite.md) for rationale.

## Transaction Usage

### Manual Transaction Control

```rust
let tx = conn.transaction()?;

// Multiple operations in transaction
let rowid1 = repo.upsert(&mut tx, &site, &post1)?;
let rowid2 = repo.upsert(&mut tx, &site, &post2)?;
repo.delete_by_post_id(&tx, &site, PostId(999))?;

tx.commit()?;
// All operations succeed or all rollback
```

### Using QueryExecutor Trait

```rust
fn sync_posts<E: QueryExecutor>(
    executor: &E,
    site: &DbSite,
    posts: Vec<AnyPostWithEditContext>,
) -> Result<()> {
    let repo = PostRepository;

    for post in posts {
        repo.upsert(executor, site, &post)?;
    }

    Ok(())
}

// Works with Connection
sync_posts(&conn, &site, posts)?;

// Also works with Transaction
let tx = conn.transaction()?;
sync_posts(&tx, &site, posts)?;
tx.commit()?;
```

## Return Types

### Domain Entity vs Wrapper

**Upsert operations** accept domain entity:
```rust
fn upsert(&self, ..., post: &AnyPostWithEditContext, ...) -> Result<RowId>
```

**Query operations** return wrapper type:
```rust
fn select_by_rowid(&self, ...) -> Result<DbAnyPostWithEditContext>
```

**Wrapper type includes:**
- `row_id: RowId` - Database rowid
- `site: DbSite` - Site reference
- `post: AnyPostWithEditContext` - Domain entity
- `last_fetched_at: String` - Cache timestamp

See [Entity vs Wrapper Types](../design-decisions/05-entity-vs-wrapper.md) for rationale.

## Error Handling

All methods return `Result<T, SqliteDbError>`:

```rust
match repo.select_by_post_id(&conn, &site, post_id) {
    Ok(db_post) => {
        println!("Found: {}", db_post.post.title.raw);
    }
    Err(SqliteDbError::NotFound) => {
        println!("Post not found");
    }
    Err(e) => {
        eprintln!("Database error: {}", e);
    }
}
```

**Common errors:**
- `SqliteDbError::NotFound` - Query returned no rows
- `SqliteDbError::ForeignKeyViolation` - Invalid site reference
- `SqliteDbError::UniqueViolation` - Duplicate `(db_site_id, id)` on insert
- `SqliteDbError::Query(_)` - Other SQL errors

## Performance Considerations

### Indexes

Queries are optimized with indexes:

```sql
-- Primary key (rowid)
CREATE UNIQUE INDEX idx_posts_edit_context_unique_db_site_id_and_id
  ON posts_edit_context(db_site_id, id);

-- Site scoping
CREATE INDEX idx_posts_edit_context_db_site_id
  ON posts_edit_context(db_site_id);
```

**Query performance:**
- `select_by_rowid` - O(log n) via primary key
- `select_by_post_id` - O(log n) via composite unique index
- `select_all` - O(n) with site filter (index scan)
- `select_by_author` - O(n) with site + author filter
- `select_by_status` - O(n) with site + status filter

### Batch Operations

Use `upsert_batch` or transactions for multiple operations:

```rust
// ❌ Slow - individual upserts
for post in posts {
    repo.upsert(&mut conn, &site, &post)?;
}

// ✅ Fast - batch with transaction
repo.upsert_batch(&mut conn, &site, &posts)?;
```

### Term Relationships

Reading posts with terms requires JOINs:
- Acceptable overhead for better queryability
- Indexes optimize JOIN performance
- Alternative (JSON arrays) would be slower for "posts by term" queries

## Related Documentation

- [Core Traits](../architecture/core-traits.md) - `Repository` trait definition
- [Type System](../architecture/type-system.md) - `RowId`, `DbSite`, wrapper types
- [Database Schema](../architecture/database-schema.md) - `posts_edit_context` table
- [TermRelationshipRepository](term-relationship-repository.md) - Term management
- [Usage Examples](../usage-examples.md) - Common patterns

## See Also

- [UPSERT Pattern](../design-decisions/06-upsert-pattern.md)
- [Multi-Site with DbSite](../design-decisions/07-multi-site-dbsite.md)
- [Cache Freshness](../design-decisions/10-cache-freshness.md)
