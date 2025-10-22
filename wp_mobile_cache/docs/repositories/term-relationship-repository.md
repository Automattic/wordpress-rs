# TermRelationshipRepository API

> **Last Updated:** 2025-10-22

Complete API documentation for the `TermRelationshipRepository` type, which manages term associations for WordPress objects (posts, pages, etc.).

## Overview

`TermRelationshipRepository` provides normalized storage and management of term relationships:
- Categories, tags, and custom taxonomies
- Reusable across object types (posts, pages, nav items)
- Observer-friendly sync (only actual changes generate events)
- Efficient queries ("find posts with tag X")

## Type Definition

```rust
pub struct TermRelationshipRepository;
```

**Zero-sized struct** - No fields, no construction overhead.

## Core Type

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

See [Type System](../architecture/type-system.md) for details.

## Sync Operations

### sync_terms_for_object

Synchronize terms for an object (only insert new, delete removed, keep unchanged).

```rust
pub fn sync_terms_for_object(
    &self,
    executor: &impl QueryExecutor,
    site: &DbSite,
    object_id: RowId,
    taxonomy_type: &TaxonomyType,
    new_term_ids: &[TermId],
) -> Result<(), SqliteDbError>
```

**Parameters:**
- `executor` - Database connection or transaction
- `site` - Site scope
- `object_id` - Database rowid of object (post, page, etc.)
- `taxonomy_type` - Type of taxonomy (Category, PostTag, Custom)
- `new_term_ids` - New term IDs that should be associated

**Returns:**
- `Result<()>` - Success or error

**Example:**
```rust
let repo = TermRelationshipRepository;
let post_rowid = RowId(42);

// Sync categories
repo.sync_terms_for_object(
    &conn,
    &site,
    post_rowid,
    &TaxonomyType::Category,
    &[TermId(1), TermId(2), TermId(3)],
)?;

// Sync tags
repo.sync_terms_for_object(
    &conn,
    &site,
    post_rowid,
    &TaxonomyType::PostTag,
    &[TermId(10), TermId(20)],
)?;
```

**Sync Behavior:**

```rust
// Current terms: [1, 2, 5]
// New terms: [1, 3, 5]

repo.sync_terms_for_object(&conn, &site, object_id, &taxonomy, &[TermId(1), TermId(3), TermId(5)])?;

// Database operations:
// - DELETE term 2 (removed)
// - INSERT term 3 (added)
// - No operation for terms 1, 5 (unchanged)

// Observer sees:
// - Action::Delete for term 2
// - Action::Insert for term 3
// - Nothing for terms 1, 5
```

**Notes:**
- Observer-friendly: only actual changes generate events
- Idempotent: calling with same terms multiple times is safe
- Efficient: calculates diff before modifying database
- Atomic: use transaction for multiple taxonomies

See [Term Normalization](../design-decisions/09-term-normalization.md) for rationale.

## Query Operations

### get_terms_for_object

Get all term IDs for an object's taxonomy.

```rust
pub fn get_terms_for_object(
    &self,
    executor: &impl QueryExecutor,
    site: &DbSite,
    object_id: RowId,
    taxonomy_type: &TaxonomyType,
) -> Result<Vec<TermId>, SqliteDbError>
```

**Parameters:**
- `executor` - Database connection or transaction
- `site` - Site scope
- `object_id` - Database rowid of object
- `taxonomy_type` - Type of taxonomy

**Returns:**
- `Vec<TermId>` - Term IDs in no particular order

**Example:**
```rust
let repo = TermRelationshipRepository;
let post_rowid = RowId(42);

// Get categories
let categories = repo.get_terms_for_object(
    &conn,
    &site,
    post_rowid,
    &TaxonomyType::Category,
)?;
println!("Categories: {:?}", categories);  // [TermId(1), TermId(2)]

// Get tags
let tags = repo.get_terms_for_object(
    &conn,
    &site,
    post_rowid,
    &TaxonomyType::PostTag,
)?;
println!("Tags: {:?}", tags);  // [TermId(10), TermId(20), TermId(30)]
```

**Notes:**
- Returns empty vec if no terms
- Fast query (uses index on `(db_site_id, object_id)`)
- Used internally by `sync_terms_for_object`

### get_all_terms_for_object

Get all term IDs grouped by taxonomy for an object.

```rust
pub fn get_all_terms_for_object(
    &self,
    executor: &impl QueryExecutor,
    site: &DbSite,
    object_id: RowId,
) -> Result<HashMap<TaxonomyType, Vec<TermId>>, SqliteDbError>
```

**Parameters:**
- `executor` - Database connection or transaction
- `site` - Site scope
- `object_id` - Database rowid of object

**Returns:**
- `HashMap<TaxonomyType, Vec<TermId>>` - All terms grouped by taxonomy

**Example:**
```rust
let repo = TermRelationshipRepository;
let post_rowid = RowId(42);

let terms_map = repo.get_all_terms_for_object(&conn, &site, post_rowid)?;

// Access categories
if let Some(categories) = terms_map.get(&TaxonomyType::Category) {
    println!("Categories: {:?}", categories);
}

// Access tags
if let Some(tags) = terms_map.get(&TaxonomyType::PostTag) {
    println!("Tags: {:?}", tags);
}

// Access custom taxonomy
let product_cat = TaxonomyType::Custom("product_category".into());
if let Some(terms) = terms_map.get(&product_cat) {
    println!("Product categories: {:?}", terms);
}
```

**Notes:**
- Single query for all taxonomies (efficient)
- Returns empty HashMap if no terms
- Used by `PostRepository::select_by_rowid` to populate post terms

## Delete Operations

### delete_all_terms_for_object

Delete all term relationships for an object.

```rust
pub fn delete_all_terms_for_object(
    &self,
    executor: &impl QueryExecutor,
    site: &DbSite,
    object_id: RowId,
) -> Result<usize, SqliteDbError>
```

**Parameters:**
- `executor` - Database connection or transaction
- `site` - Site scope
- `object_id` - Database rowid of object

**Returns:**
- `usize` - Number of relationships deleted

**Example:**
```rust
let repo = TermRelationshipRepository;
let post_rowid = RowId(42);

let deleted = repo.delete_all_terms_for_object(&conn, &site, post_rowid)?;
println!("Deleted {} term relationships", deleted);
```

**Notes:**
- Deletes across all taxonomies (categories, tags, custom)
- Returns 0 if no relationships exist (not an error)
- Used by `PostRepository::delete_by_post_id` before deleting post

## Custom Taxonomies

### Registering Custom Taxonomy

```rust
use wp_api::taxonomies::TaxonomyType;

// Built-in taxonomies
let category = TaxonomyType::Category;
let post_tag = TaxonomyType::PostTag;

// Custom taxonomy
let product_cat = TaxonomyType::Custom("product_category".into());
let post_format = TaxonomyType::Custom("post_format".into());
```

### Using Custom Taxonomy

```rust
let repo = TermRelationshipRepository;
let product_post_rowid = RowId(100);

// Sync product categories
repo.sync_terms_for_object(
    &conn,
    &site,
    product_post_rowid,
    &TaxonomyType::Custom("product_category".into()),
    &[TermId(50), TermId(51), TermId(52)],
)?;

// Query product categories
let product_categories = repo.get_terms_for_object(
    &conn,
    &site,
    product_post_rowid,
    &TaxonomyType::Custom("product_category".into()),
)?;
```

## Querying Objects by Term

### Future API (Not Yet Implemented)

```rust
// Find all posts with specific term
pub fn get_objects_by_term(
    &self,
    executor: &impl QueryExecutor,
    site: &DbSite,
    taxonomy_type: &TaxonomyType,
    term_id: TermId,
) -> Result<Vec<RowId>, SqliteDbError>
```

**Example usage:**
```rust
// Find all posts with category "News" (TermId(5))
let object_ids = term_repo.get_objects_by_term(
    &conn,
    &site,
    &TaxonomyType::Category,
    TermId(5),
)?;

// Load full posts
let posts: Vec<DbAnyPostWithEditContext> = object_ids
    .iter()
    .map(|&rowid| post_repo.select_by_rowid(&conn, &site, rowid))
    .collect::<Result<_, _>>()?;
```

**Why this enables better queries:**

```sql
-- Efficient query with index
SELECT object_id FROM term_relationships
WHERE db_site_id = ? AND taxonomy_type = 'category' AND term_id = 5;
-- Uses: idx_term_relationships_by_term

-- vs JSON array approach (slow)
SELECT rowid, categories FROM posts_edit_context WHERE db_site_id = ?;
-- Must parse JSON for every post, no index
```

## Database Schema

### Table

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
-- Prevent duplicate associations
CREATE UNIQUE INDEX idx_term_relationships_unique
  ON term_relationships(db_site_id, object_id, term_id, taxonomy_type);

-- Query: "Find all objects with taxonomy X and term Y"
CREATE INDEX idx_term_relationships_by_term
  ON term_relationships(db_site_id, taxonomy_type, term_id);

-- Query: "Find all terms for object X"
CREATE INDEX idx_term_relationships_by_object
  ON term_relationships(db_site_id, object_id);
```

See [Database Schema](../architecture/database-schema.md) for details.

## Transaction Usage

### Atomic Term Sync

```rust
let tx = conn.transaction()?;
let repo = TermRelationshipRepository;

// Sync multiple taxonomies atomically
repo.sync_terms_for_object(&tx, &site, object_id, &TaxonomyType::Category, &categories)?;
repo.sync_terms_for_object(&tx, &site, object_id, &TaxonomyType::PostTag, &tags)?;

tx.commit()?;
// Both succeed or both rollback
```

### Post + Terms in Transaction

```rust
// PostRepository.upsert() handles term syncing automatically
let post_rowid = post_repo.upsert(&mut conn, &site, &post)?;
```

**Note:** `PostRepository::upsert()` creates a transaction internally and syncs both the post and its term relationships atomically. The categories and tags from `post.categories` and `post.tags` are automatically synchronized using `TermRelationshipRepository::sync_terms_for_object()`.

## Extensibility

### Supporting Pages

```rust
// TermRelationshipRepository is reusable
let page_rowid = page_repo.upsert(&conn, &site, &page)?;

// Same API works for pages
term_repo.sync_terms_for_object(
    &conn,
    &site,
    page_rowid,
    &TaxonomyType::Category,
    &page.categories,
)?;
```

### Supporting Nav Menu Items

```rust
// Works with any object type
let nav_item_rowid = nav_repo.upsert(&conn, &site, &nav_item)?;

term_repo.sync_terms_for_object(
    &conn,
    &site,
    nav_item_rowid,
    &TaxonomyType::Custom("nav_menu".into()),
    &[TermId(menu_id)],
)?;
```

**Why `object_id` has no foreign key:**

The `object_id` field can reference different tables:
- `posts_edit_context.rowid`
- `pages_edit_context.rowid`
- `nav_menu_items.rowid`

Cannot create a single foreign key to multiple tables. Data integrity maintained by application logic.

## Performance Considerations

### Sync Efficiency

Sync calculates diff before modifying database:

```rust
// Existing terms: [1, 2, 3, 4, 5]
// New terms: [1, 3, 5, 6, 7]

// Only these operations:
// DELETE terms 2, 4
// INSERT terms 6, 7

// Terms 1, 3, 5 - no operations
```

**Benefits:**
- Minimizes database writes
- Reduces observer events
- Preserves rowids for unchanged relationships

### Index Usage

Queries are optimized with indexes:

- `sync_terms_for_object`: Uses `idx_term_relationships_by_object`
- `get_terms_for_object`: Uses `idx_term_relationships_by_object`
- `get_objects_by_term`: Uses `idx_term_relationships_by_term`

### Batch Operations

Use transactions for multiple syncs:

```rust
// ❌ Slow - individual transactions
for post in posts {
    let rowid = post_repo.upsert(&conn, &site, &post)?;
    term_repo.sync_terms_for_object(&conn, &site, rowid, &taxonomy, &terms)?;
}

// ✅ Fast - single transaction
let tx = conn.transaction()?;
for post in posts {
    let rowid = post_repo.upsert(&tx, &site, &post)?;
    term_repo.sync_terms_for_object(&tx, &site, rowid, &taxonomy, &terms)?;
}
tx.commit()?;
```

## Error Handling

```rust
match term_repo.sync_terms_for_object(&conn, &site, object_id, &taxonomy, &terms) {
    Ok(()) => {
        println!("Terms synced successfully");
    }
    Err(SqliteDbError::ForeignKeyViolation) => {
        eprintln!("Invalid site or object_id");
    }
    Err(SqliteDbError::UniqueViolation) => {
        eprintln!("Duplicate term relationship (should not happen with sync)");
    }
    Err(e) => {
        eprintln!("Database error: {}", e);
    }
}
```

## Related Documentation

- [PostRepository](post-repository.md) - Integration with posts
- [Term Normalization](../design-decisions/09-term-normalization.md) - Design rationale
- [Database Schema](../architecture/database-schema.md) - Table structure
- [Type System](../architecture/type-system.md) - `DbTermRelationship` type

## See Also

- [Usage Examples](../usage-examples.md) - Common patterns
- [Migration Guide](../migration-guide.md) - Adding new entities with terms
