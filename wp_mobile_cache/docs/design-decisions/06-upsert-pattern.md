# Design Decision 6: UPSERT for Insert/Update Operations

> **Last Updated:** 2025-10-21

## Decision

Use SQLite's `INSERT ... ON CONFLICT ... DO UPDATE` for atomic insert/update operations.

## Context

When caching data from WordPress, we need to handle both new posts and updates to existing posts:

```rust
// Option 1: Separate insert and update (rejected)
pub fn upsert(...) -> Result<RowId> { }
pub fn update(...) -> Result<()> { }

// Option 2: Upsert (chosen)
pub fn upsert(...) -> Result<RowId> { }
```

## Rationale

### Database Observers

**SQLite supports observing database changes:**

```rust
// Register update hook
conn.update_hook(Some(|action, database, table, rowid| {
    match action {
        Action::Insert => println!("INSERT into {}", table),
        Action::Update => println!("UPDATE on {}", table),
        Action::Delete => println!("DELETE from {}", table),
    }
}));
```

**UPSERT ensures observers see the correct action:**

```rust
// ✅ With UPSERT
repo.upsert(&conn, &site, &post)?;
// Observer sees: INSERT (new post) or UPDATE (existing post)
// Never sees: DELETE + INSERT

// ❌ With manual delete-then-insert
conn.execute("DELETE FROM posts WHERE id = ?", [post.id])?;
conn.execute("INSERT INTO posts ...", [...])?;
// Observer sees: DELETE + INSERT
// Loses information that this was an update
```

**Why this matters:**
- UI can react differently to INSERT vs UPDATE
- Sync logic can optimize based on operation type
- Audit logs are more accurate
- Database triggers work correctly

### DRY Principle

**Write SQL field list only once:**

```rust
// ✅ UPSERT - field list shared between INSERT and UPDATE
pub fn upsert(...) -> Result<RowId> {
    executor.execute(
        r#"
        INSERT INTO posts_edit_context (db_site_id, id, title, content, author, ...)
        VALUES (:db_site_id, :id, :title, :content, :author, ...)
        ON CONFLICT(db_site_id, id) DO UPDATE SET
            title = excluded.title,
            content = excluded.content,
            author = excluded.author,
            ...
        "#,
        named_params! { ... }
    )
}

// ❌ Separate methods - field list duplicated
pub fn upsert(...) -> Result<RowId> {
    executor.execute(
        "INSERT INTO posts (db_site_id, id, title, content, author, ...) VALUES (?, ?, ?, ?, ?, ...)",
        params![...]
    )
}

pub fn update(...) -> Result<()> {
    executor.execute(
        "UPDATE posts SET title = ?, content = ?, author = ?, ... WHERE db_site_id = ? AND id = ?",
        params![...]
    )
}
```

**Benefits:**
- ✅ Single source of truth for field mapping
- ✅ Impossible for insert and update to get out of sync
- ✅ Easier to maintain when adding fields
- ✅ Less code to write and test

### Rowid Preservation

**UPSERT keeps the same rowid:**

```rust
// First upsert - creates new row
let rowid1 = repo.upsert(&conn, &site, &post)?;
// rowid1 = RowId(42)

// Second upsert with same post ID - updates existing row
let rowid2 = repo.upsert(&conn, &site, &updated_post)?;
// rowid2 = RowId(42) - SAME rowid

// ❌ Delete + Insert would generate new rowid
// rowid2 = RowId(43) - DIFFERENT rowid
```

**Why this matters:**
- Foreign keys remain valid (term_relationships reference rowid)
- External references to cached data stay consistent
- Reduces database churn
- Predictable behavior

### Natural Key

**Uses composite unique index for conflict detection:**

```sql
-- Unique index on (db_site_id, id)
CREATE UNIQUE INDEX idx_posts_edit_context_unique_db_site_id_and_id
  ON posts_edit_context(db_site_id, id);
```

**UPSERT leverages this index:**

```rust
executor.execute(
    r#"
    INSERT INTO posts_edit_context (db_site_id, id, ...)
    VALUES (:db_site_id, :id, ...)
    ON CONFLICT(db_site_id, id) DO UPDATE SET ...
    "#,
    named_params! { ... }
)
```

**Natural key is composite:**
- `db_site_id` - Site scope
- `id` - WordPress post ID

Same WordPress post ID can exist across different sites.

## Implementation

### Full UPSERT Example

```rust
impl PostRepository {
    pub fn upsert(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        post: &AnyPostWithEditContext,
    ) -> Result<RowId, SqliteDbError> {
        executor.execute(
            r#"
            INSERT INTO posts_edit_context (
                db_site_id, id, date, modified, slug, status, type,
                title_raw, content_raw, author, categories, tags,
                last_fetched_at
            )
            VALUES (
                :db_site_id, :id, :date, :modified, :slug, :status, :type,
                :title_raw, :content_raw, :author, :categories, :tags,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            )
            ON CONFLICT(db_site_id, id) DO UPDATE SET
                date = excluded.date,
                modified = excluded.modified,
                slug = excluded.slug,
                status = excluded.status,
                type = excluded.type,
                title_raw = excluded.title_raw,
                content_raw = excluded.content_raw,
                author = excluded.author,
                categories = excluded.categories,
                tags = excluded.tags,
                last_fetched_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            "#,
            named_params! {
                ":db_site_id": site.row_id.0,
                ":id": post.id.0,
                ":date": post.date,
                ":modified": post.modified,
                ":slug": post.slug,
                ":status": post.status,
                ":type": post.type_,
                ":title_raw": post.title.raw,
                ":content_raw": post.content.raw,
                ":author": post.author.0,
                ":categories": serialize_terms(&post.categories),
                ":tags": serialize_terms(&post.tags),
            }
        )?;
        Ok(executor.last_insert_rowid())
    }
}
```

### Key Points

**`excluded` keyword:**
- Refers to the row that would have been inserted
- Used in UPDATE SET clause to reference new values

**`last_fetched_at` handling:**
- Set to current time on both INSERT and UPDATE
- Uses SQLite's `strftime()` function for consistency

**Returns rowid:**
- `last_insert_rowid()` returns the rowid whether INSERT or UPDATE occurred
- SQLite updates this even for UPDATE operations

## Example Usage

### First Insert

```rust
let post = AnyPostWithEditContext {
    id: PostId(123),
    title: PostTitleWithEditContext { raw: "Hello".into() },
    // ...
};

let rowid = repo.upsert(&conn, &site, &post)?;
// SQL: INSERT INTO posts_edit_context ...
// Observer sees: Action::Insert
// rowid = RowId(42)
```

### Subsequent Update

```rust
let updated_post = AnyPostWithEditContext {
    id: PostId(123),  // Same ID
    title: PostTitleWithEditContext { raw: "Hello World".into() },  // Changed
    // ...
};

let rowid = repo.upsert(&conn, &site, &updated_post)?;
// SQL: UPDATE posts_edit_context SET ... WHERE rowid = 42
// Observer sees: Action::Update
// rowid = RowId(42) - Same rowid!
```

### Syncing from API

```rust
// Fetch posts from WordPress REST API
let posts = wp_api_client.get_posts().await?;

// Upsert all posts
for post in posts {
    repo.upsert(&conn, &site, &post)?;
    // Automatically handles new posts (INSERT) and updates (UPDATE)
}
```

## Alternatives Considered

### Alternative 1: Separate Insert and Update Methods

```rust
pub fn upsert(&self, executor: &impl QueryExecutor, post: &Post) -> Result<RowId> { }
pub fn update(&self, executor: &impl QueryExecutor, post: &Post) -> Result<()> { }
```

**Why rejected:**
- Caller must decide which to use
- Duplicates field list in SQL
- Observer sees incorrect action if using delete+insert pattern
- More code to maintain

### Alternative 2: Check Existence, Then Insert or Update

```rust
pub fn upsert(&self, executor: &impl QueryExecutor, post: &Post) -> Result<RowId> {
    if self.exists(&executor, post.id)? {
        self.update(&executor, post)?;
        self.select_rowid_by_post_id(&executor, post.id)
    } else {
        self.insert(&executor, post)
    }
}
```

**Why rejected:**
- Two queries instead of one (performance)
- Race condition if not in transaction
- More complex code
- Still has separate insert/update SQL

### Alternative 3: REPLACE Statement

```sql
REPLACE INTO posts_edit_context (...) VALUES (...)
```

**Why rejected:**
- `REPLACE` is actually `DELETE` + `INSERT`
- Observer sees DELETE + INSERT (not UPDATE)
- rowid changes on each replace
- Breaks foreign key references
- Loses the semantic meaning of "update"

## Trade-offs

### Advantages

✅ **Observer-friendly** - Correct INSERT/UPDATE actions
✅ **DRY** - Single SQL statement for both operations
✅ **Rowid stability** - Same rowid preserved on update
✅ **Atomic** - Single database operation
✅ **Maintainable** - Only one place to update field list
✅ **Simple API** - Caller doesn't choose insert vs update

### Disadvantages

❌ **SQLite-specific** - Not all databases support ON CONFLICT
❌ **Requires unique index** - Natural key must be indexed
❌ **Less explicit** - Caller doesn't know if insert or update occurred

**Mitigation:**
- We're committed to SQLite (acceptable)
- Natural keys should be indexed anyway (performance)
- Can check last_modified if need to distinguish

## SQLite ON CONFLICT Documentation

```sql
-- Syntax
INSERT INTO table (columns)
VALUES (values)
ON CONFLICT (conflict_columns) DO UPDATE SET
    column1 = excluded.column1,
    column2 = excluded.column2;

-- Conflict target
ON CONFLICT (db_site_id, id)  -- Composite unique constraint

-- excluded table
-- Refers to the row that would have been inserted
excluded.column_name
```

## Related Decisions

- [Database Schema](../architecture/database-schema.md) - Unique index definition
- [Cache Freshness](10-cache-freshness.md) - `last_fetched_at` in UPSERT
- [Multi-Site with DbSite](07-multi-site-dbsite.md) - Composite key design

## References

- [SQLite ON CONFLICT](https://www.sqlite.org/lang_conflict.html)
- [SQLite UPSERT](https://www.sqlite.org/lang_upsert.html)

## See Also

- [PostRepository](../repositories/post-repository.md) - Upsert implementation
- [Usage Examples](../usage-examples.md) - Upsert usage patterns
