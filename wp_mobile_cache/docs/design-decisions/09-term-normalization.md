# Design Decision 9: Term Relationships Normalization

> **Last Updated:** 2025-10-21

## Decision

Store post terms (categories, tags, custom taxonomies) in a normalized `term_relationships` table instead of JSON arrays.

## Context

WordPress posts have associated terms in different taxonomies:
- Categories (e.g., `[1, 2, 5]`)
- Tags (e.g., `[10, 20, 30]`)
- Custom taxonomies

We need to decide how to store these associations in the cache.

## Rationale

### Better Queryability

**Enable efficient queries like "find all posts with tag X":**

```rust
// ✅ With normalized table
pub fn select_by_term(
    &self,
    executor: &impl QueryExecutor,
    site: &DbSite,
    taxonomy: &TaxonomyType,
    term_id: TermId,
) -> Result<Vec<DbAnyPostWithEditContext>> {
    let sql = r#"
        SELECT p.* FROM posts_edit_context p
        INNER JOIN term_relationships tr ON p.rowid = tr.object_id
        WHERE tr.db_site_id = ?
          AND tr.taxonomy_type = ?
          AND tr.term_id = ?
    "#;
    // Fast - uses index on (db_site_id, taxonomy_type, term_id)
}

// ❌ With JSON array
pub fn select_by_term(...) -> Result<Vec<DbAnyPostWithEditContext>> {
    // Must read ALL posts and parse JSON for each
    let all_posts = self.select_all(executor, site)?;
    all_posts.into_iter()
        .filter(|post| {
            // Parse JSON array from TEXT column
            let tags: Vec<TermId> = serde_json::from_str(&post.tags_json)?;
            tags.contains(&term_id)
        })
        .collect()
    // Slow - no index, full table scan, JSON parsing
}
```

**SQL queries enabled:**
- Find posts by category
- Find posts by tag
- Find posts with multiple terms
- Count posts per term
- Find related posts (shared terms)

### Referential Integrity

**Foreign key on `db_site_id` ensures site-level data consistency:**

```sql
CREATE TABLE `term_relationships` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `db_site_id` INTEGER NOT NULL,
  `object_id` INTEGER NOT NULL,
  `term_id` INTEGER NOT NULL,
  `taxonomy_type` TEXT NOT NULL,

  FOREIGN KEY (db_site_id) REFERENCES sites(id) ON DELETE CASCADE
) STRICT;
```

**Benefits:**
- ✅ Cannot create term relationship for invalid site
- ✅ Deleting site cascades to term relationships
- ✅ Data integrity enforced by database

**Why no FK on `object_id`?**

The `object_id` can reference different tables:
- `posts_edit_context`
- `pages_edit_context` (future)
- `nav_menu_items` (future)

Cannot create a single foreign key to multiple tables. Integrity maintained by application logic.

### Matches WordPress

**Table name mirrors `wp_term_relationships` for familiarity:**

WordPress database schema:

```sql
-- WordPress core table
CREATE TABLE wp_term_relationships (
  object_id bigint(20) unsigned NOT NULL,
  term_taxonomy_id bigint(20) unsigned NOT NULL,
  term_order int(11) NOT NULL,
  PRIMARY KEY (object_id, term_taxonomy_id)
);
```

Our cache table:

```sql
CREATE TABLE term_relationships (
  rowid INTEGER PRIMARY KEY AUTOINCREMENT,
  db_site_id INTEGER NOT NULL,
  object_id INTEGER NOT NULL,      -- rowid of post/page/etc
  term_id INTEGER NOT NULL,         -- WordPress term ID
  taxonomy_type TEXT NOT NULL,      -- 'category', 'post_tag', custom
  FOREIGN KEY (db_site_id) REFERENCES sites(id) ON DELETE CASCADE
);
```

**Differences:**
- ✅ Added `db_site_id` for multi-site support
- ✅ Added `taxonomy_type` to denormalize taxonomy lookup
- ✅ Use `term_id` directly instead of `term_taxonomy_id`

**Why denormalize taxonomy_type?**

WordPress stores taxonomy in a separate `term_taxonomy` table. We denormalize for simplicity:
- One less JOIN in common queries
- Cache is read-heavy, write-light
- Taxonomy type rarely changes

### Extensible

**Supports any taxonomy type and object type:**

```rust
use wp_api::taxonomies::TaxonomyType;

pub enum TaxonomyType {
    Category,
    PostTag,
    Custom(String),  // e.g., "product_category", "post_format"
}

// Works with any taxonomy
term_repo.sync_terms_for_object(
    &tx,
    site,
    object_id,
    &TaxonomyType::Category,
    &category_ids,
)?;

term_repo.sync_terms_for_object(
    &tx,
    site,
    object_id,
    &TaxonomyType::Custom("product_category".into()),
    &product_category_ids,
)?;
```

**Works with any object type:**

```rust
// Posts
let post_rowid = post_repo.upsert(&tx, site, &post)?;
term_repo.sync_terms_for_object(&tx, site, post_rowid, &taxonomy, &terms)?;

// Pages (future)
let page_rowid = page_repo.upsert(&tx, site, &page)?;
term_repo.sync_terms_for_object(&tx, site, page_rowid, &taxonomy, &terms)?;

// Nav menu items (future)
let nav_rowid = nav_repo.upsert(&tx, site, &nav_item)?;
term_repo.sync_terms_for_object(&tx, site, nav_rowid, &taxonomy, &terms)?;
```

### Observer-Friendly

**Sync approach only generates events for actual changes:**

```rust
pub fn sync_terms_for_object(
    &self,
    executor: &impl QueryExecutor,
    site: &DbSite,
    object_id: RowId,
    taxonomy_type: &TaxonomyType,
    new_term_ids: &[TermId],
) -> Result<()> {
    // 1. Get existing terms
    let existing = self.get_terms_for_object(executor, site, object_id, taxonomy_type)?;

    // 2. Calculate diff
    let to_delete: Vec<_> = existing.iter()
        .filter(|&term_id| !new_term_ids.contains(term_id))
        .collect();
    let to_insert: Vec<_> = new_term_ids.iter()
        .filter(|&term_id| !existing.contains(term_id))
        .collect();

    // 3. Delete removed terms
    for term_id in to_delete {
        // Observer sees: Action::Delete
    }

    // 4. Insert new terms
    for term_id in to_insert {
        // Observer sees: Action::Insert
    }

    // 5. Unchanged terms: NO operations = NO observer events
}
```

**Compare with JSON array approach:**

```rust
// ❌ JSON approach - always generates update event
pub fn update_post_with_terms(...) {
    // Update entire post row (including JSON arrays)
    executor.execute(
        "UPDATE posts SET tags = ?, categories = ? WHERE id = ?",
        [tags_json, categories_json, post_id]
    )?;
    // Observer sees: Action::Update (even if terms unchanged)
}
```

**Observer benefits:**
- ✅ Only see events for actual term additions/removals
- ✅ Can react specifically to term changes
- ✅ More efficient change tracking

## Database Schema

### Table Definition

```sql
CREATE TABLE `term_relationships` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `db_site_id` INTEGER NOT NULL,
  `object_id` INTEGER NOT NULL,      -- rowid of post/page/nav_menu_item/etc
  `term_id` INTEGER NOT NULL,         -- WordPress term ID
  `taxonomy_type` TEXT NOT NULL,      -- 'category', 'post_tag', or custom taxonomy

  FOREIGN KEY (db_site_id) REFERENCES sites(id) ON DELETE CASCADE
) STRICT;
```

### Indexes

```sql
-- Prevent duplicate associations (same object can't have same term twice in same taxonomy)
CREATE UNIQUE INDEX idx_term_relationships_unique
  ON term_relationships(db_site_id, object_id, term_id, taxonomy_type);

-- Query: "Find all objects with taxonomy X and term Y"
CREATE INDEX idx_term_relationships_by_term
  ON term_relationships(db_site_id, taxonomy_type, term_id);

-- Query: "Find all terms for object X" (used in joins when reading posts)
CREATE INDEX idx_term_relationships_by_object
  ON term_relationships(db_site_id, object_id);
```

**Index strategy:**
- Unique index prevents duplicate associations
- `by_term` index optimizes "posts with tag X" queries
- `by_object` index optimizes reading post with terms

## Type Design

```rust
use wp_api::terms::TermId;
use wp_api::taxonomies::TaxonomyType;

pub struct DbTermRelationship {
    pub row_id: RowId,
    pub site: DbSite,
    pub object_id: RowId,           // rowid of post/page/etc
    pub term_id: TermId,             // WordPress term ID
    pub taxonomy_type: TaxonomyType, // Category, PostTag, or Custom
}
```

**Uses types from `wp_api` crate:**
- `TermId` - WordPress term ID (type-safe wrapper around i64)
- `TaxonomyType` - Enum for taxonomy types

## Repository Design

### TermRelationshipRepository

Generic term management (reusable for posts, pages, etc.):

```rust
pub struct TermRelationshipRepository;

impl TermRelationshipRepository {
    /// Synchronize terms for an object (only insert new, delete removed, keep unchanged)
    pub fn sync_terms_for_object(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        object_id: RowId,
        taxonomy_type: &TaxonomyType,
        new_term_ids: &[TermId],
    ) -> Result<()> {
        // Diff-based sync
    }

    /// Get all term IDs for an object's taxonomy
    pub fn get_terms_for_object(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        object_id: RowId,
        taxonomy_type: &TaxonomyType,
    ) -> Result<Vec<TermId>> {
        // Query term_relationships
    }

    /// Get all term IDs grouped by taxonomy for an object
    pub fn get_all_terms_for_object(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        object_id: RowId,
    ) -> Result<HashMap<TaxonomyType, Vec<TermId>>> {
        // Query all taxonomies at once
    }

    /// Delete all terms for an object
    pub fn delete_all_terms_for_object(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        object_id: RowId,
    ) -> Result<usize> {
        // Delete all term relationships
    }
}
```

See [TermRelationshipRepository API](../repositories/term-relationship-repository.md) for full documentation.

### PostRepository Integration

```rust
impl PostRepository {
    pub fn upsert_with_terms(
        &self,
        transaction_manager: &mut impl TransactionManager,
        site: &DbSite,
        post: &AnyPostWithEditContext,
    ) -> Result<RowId> {
        let tx = transaction_manager.transaction()?;

        // Upsert the post
        let post_rowid = self.upsert(&tx, site, post)?;

        // Sync term relationships
        let term_repo = TermRelationshipRepository;

        if let Some(ref categories) = post.categories {
            term_repo.sync_terms_for_object(
                &tx, site, post_rowid, &TaxonomyType::Category, categories
            )?;
        }

        if let Some(ref tags) = post.tags {
            term_repo.sync_terms_for_object(
                &tx, site, post_rowid, &TaxonomyType::PostTag, tags
            )?;
        }

        tx.commit()?;
        Ok(post_rowid)
    }
}
```

## Example Usage

### Syncing Terms

```rust
let mut post = AnyPostWithEditContext {
    id: PostId(123),
    categories: Some(vec![TermId(1), TermId(2)]),
    tags: Some(vec![TermId(10), TermId(20), TermId(30)]),
    // ...
};

// First upsert - inserts terms
let post_rowid = repo.upsert_with_terms(&mut conn, &site, &post)?;
// Database now has:
// - term_relationships: (object_id=post_rowid, term_id=1, taxonomy=Category)
// - term_relationships: (object_id=post_rowid, term_id=2, taxonomy=Category)
// - term_relationships: (object_id=post_rowid, term_id=10, taxonomy=PostTag)
// - term_relationships: (object_id=post_rowid, term_id=20, taxonomy=PostTag)
// - term_relationships: (object_id=post_rowid, term_id=30, taxonomy=PostTag)

// Update with different terms
post.categories = Some(vec![TermId(1), TermId(3)]);  // Removed 2, added 3
post.tags = Some(vec![TermId(10)]);                   // Removed 20, 30

repo.upsert_with_terms(&mut conn, &site, &post)?;
// Observer sees:
// - DELETE for term 2 (category)
// - INSERT for term 3 (category)
// - DELETE for terms 20, 30 (tags)
// Observer does NOT see events for terms 1, 10 (unchanged)
```

### Querying by Term

```rust
// Find all posts with category 1
let posts = repo.select_by_term(
    &conn,
    &site,
    &TaxonomyType::Category,
    TermId(1),
)?;

// Find all posts with tag 10
let posts = repo.select_by_term(
    &conn,
    &site,
    &TaxonomyType::PostTag,
    TermId(10),
)?;
```

## Alternatives Considered

### Alternative 1: JSON Arrays in Post Table

```sql
CREATE TABLE posts_edit_context (
  -- ...
  categories TEXT,  -- JSON: "[1, 2, 5]"
  tags TEXT,        -- JSON: "[10, 20, 30]"
);
```

**Why rejected:**
- ❌ Cannot query "posts with tag X" efficiently
- ❌ No indexes possible on JSON array elements
- ❌ Must parse JSON for every query
- ❌ Full table scan required
- ❌ Cannot enforce referential integrity
- ❌ Less flexible for complex queries

### Alternative 2: Comma-Separated IDs

```sql
CREATE TABLE posts_edit_context (
  -- ...
  categories TEXT,  -- "1,2,5"
  tags TEXT,        -- "10,20,30"
);
```

**Why rejected:**
- ❌ Same problems as JSON arrays
- ❌ Even harder to parse
- ❌ Type-unsafe (strings, not numbers)

### Alternative 3: Separate Category and Tag Tables

```sql
CREATE TABLE post_categories (
  post_id INTEGER,
  category_id INTEGER
);

CREATE TABLE post_tags (
  post_id INTEGER,
  tag_id INTEGER
);
```

**Why rejected:**
- ❌ Doesn't scale to custom taxonomies
- ❌ Need new table for each taxonomy
- ❌ Code duplication

### Alternative 4: Store Term Objects, Not Just IDs

```sql
CREATE TABLE term_relationships (
  -- ...
  term_data TEXT  -- JSON: {"id": 1, "name": "News", "slug": "news"}
);
```

**Why rejected:**
- ❌ Data duplication (term name repeated for each post)
- ❌ Update anomaly (changing term name requires updating all posts)
- ❌ Should cache terms separately in `terms` table

## Trade-offs

### Advantages

✅ **Queryable** - Efficient "posts by term" queries
✅ **Indexed** - Database indexes optimize lookups
✅ **Extensible** - Supports any taxonomy and object type
✅ **Observer-friendly** - Only actual changes generate events
✅ **Referential integrity** - Foreign key constraints
✅ **WordPress-familiar** - Mirrors wp_term_relationships
✅ **Type-safe** - Uses wp_api types

### Disadvantages

❌ **More complex** - Separate table and repository
❌ **More JOINs** - Reading post requires joining terms
❌ **Write overhead** - Must sync term relationships separately

**Mitigation:**
- Complexity is encapsulated in repository
- JOINs are fast with proper indexes
- Write overhead is acceptable for better read performance and queryability

## Related Decisions

- [Database Schema](../architecture/database-schema.md) - Term relationships table
- [Type System](../architecture/type-system.md) - DbTermRelationship type
- [Multi-Site with DbSite](07-multi-site-dbsite.md) - Site scoping

## See Also

- [TermRelationshipRepository](../repositories/term-relationship-repository.md) - Full API documentation
- [PostRepository](../repositories/post-repository.md) - Integration with terms
- [Usage Examples](../usage-examples.md) - Term management patterns
