# Design Decision 7: Multi-Site Architecture with DbSite

> **Last Updated:** 2025-10-21

## Decision

All entities are scoped to a site via `DbSite` parameter with foreign key constraints.

## Context

WordPress mobile apps often manage multiple sites (self-hosted and WordPress.com). The cache needs to:
- Isolate data between sites
- Prevent post ID collisions (same ID on different sites)
- Provide type-safe site references

## Rationale

### Data Isolation

**Posts from different sites cannot conflict:**

```sql
-- Same WordPress post ID allowed per site
INSERT INTO posts_edit_context (db_site_id, id, ...) VALUES (1, 123, ...);  -- Site 1, Post 123
INSERT INTO posts_edit_context (db_site_id, id, ...) VALUES (2, 123, ...);  -- Site 2, Post 123
-- Both succeed - different sites
```

**Unique constraint is composite:**

```sql
CREATE UNIQUE INDEX idx_posts_edit_context_unique_db_site_id_and_id
  ON posts_edit_context(db_site_id, id);
```

This ensures:
- ✅ Same post ID can exist on multiple sites
- ✅ Each post ID is unique within a site
- ❌ Cannot insert duplicate (site, post_id) combination

### Referential Integrity

**Foreign key ensures posts cannot exist without a valid site:**

```sql
CREATE TABLE `posts_edit_context` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `db_site_id` INTEGER NOT NULL,
  `id` INTEGER NOT NULL,
  -- ... other fields

  FOREIGN KEY (db_site_id) REFERENCES sites(id) ON DELETE CASCADE
) STRICT;
```

**Benefits:**
- ✅ Cannot insert post with invalid site ID
- ✅ Deleting a site cascades to all its posts
- ✅ Database enforces data integrity
- ✅ No orphaned posts

**Example:**

```rust
// ❌ This fails - site doesn't exist
let fake_site = DbSite { row_id: RowId(999) };
repo.upsert(&conn, &post, &fake_site)?;
// Error: FOREIGN KEY constraint failed

// ✅ Must use valid site
let site = site_repo.get_or_create(&conn, &site_info)?;
repo.upsert(&conn, &post, &site)?;
```

### Query Scoping

**All queries automatically filter by site:**

```rust
impl PostRepository {
    pub fn select_all(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,  // Required parameter
    ) -> Result<Vec<DbAnyPostWithEditContext>> {
        let sql = "SELECT * FROM posts_edit_context WHERE db_site_id = ?";
        //                                            ^^^^^^^^^^^^^^^^^^^^
        //                                            Always filter by site
    }
}
```

**Prevents cross-site data leaks:**

```rust
let site1 = DbSite { row_id: RowId(1) };
let site2 = DbSite { row_id: RowId(2) };

// Query site 1 - only gets site 1 posts
let posts1 = repo.select_all(&conn, &site1)?;

// Query site 2 - only gets site 2 posts
let posts2 = repo.select_all(&conn, &site2)?;

// No way to accidentally mix sites
```

### Cascade Deletion

**Deleting a site automatically removes all associated data:**

```rust
// Delete site
site_repo.delete(&conn, site)?;

// Automatically cascades to:
// - All posts for the site
// - All term relationships for those posts
// - Any other entities with FK to sites

// No orphaned data, no manual cleanup needed
```

## Why `DbSite` Instead of a Simple ID?

### 1. Prevents Confusion with WordPress Identifiers

**`DbSite` is clearly a database-internal type:**

```rust
pub struct DbSite {
    pub row_id: RowId,  // Database rowid - clear this is internal
}

// ❌ Ambiguous - what kind of ID?
fn select_all(site_id: i64) -> Result<Vec<Post>> { }

// ✅ Clear - database site identifier
fn select_all(site: &DbSite) -> Result<Vec<Post>> { }
```

**Cannot be confused with:**
- WordPress.com site IDs (numeric IDs from WordPress.com API)
- Site domains (string identifiers)
- Any other external identifier

**The `Db` prefix signals:** "This is an internal database identifier, not a WordPress concept."

### 2. Forces Valid Site References

**Callers must fetch a valid `DbSite` from site repository first:**

```rust
// ❌ Bad practice - arbitrary ID construction
let site = DbSite { row_id: RowId(999) };  // Does site 999 exist?
repo.upsert(&conn, &post, &site)?;  // Might fail with FK error

// ✅ Correct approach - fetch valid site
let site = site_repo.get_or_create(&conn, &site_info)?;
repo.upsert(&conn, &post, &site)?;  // Guaranteed to succeed (if post valid)
```

**Benefits:**
- Site existence verified before use
- Foreign key errors caught at the point of site retrieval, not later
- Clearer error messages
- Encourages proper architecture

### 3. Future-Proof for Site Polymorphism

**When site type tables are added, `DbSite` will gain fields:**

```rust
// Current implementation
pub struct DbSite {
    pub row_id: RowId,
}

// Future enhancement
pub struct DbSite {
    pub row_id: RowId,
    pub site_type: SiteType,      // SelfHosted | WordPressCom
    pub mapped_site_id: RowId,    // FK to specific site type table
}

pub enum SiteType {
    SelfHosted,
    WordPressCom,
}
```

**Database schema for site polymorphism:**

```sql
-- Base sites table
CREATE TABLE sites (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    site_type TEXT NOT NULL  -- 'self_hosted' or 'wordpress_com'
);

-- Self-hosted site details
CREATE TABLE self_hosted_sites (
    id INTEGER PRIMARY KEY,
    domain TEXT NOT NULL,
    FOREIGN KEY (id) REFERENCES sites(id) ON DELETE CASCADE
);

-- WordPress.com site details
CREATE TABLE wordpress_com_sites (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL,  -- WordPress.com site ID
    domain TEXT NOT NULL,
    FOREIGN KEY (id) REFERENCES sites(id) ON DELETE CASCADE
);
```

**Enhanced `DbSite` enables type-aware queries:**

```rust
impl PostRepository {
    pub fn select_with_site_info(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
    ) -> Result<Vec<PostWithSiteInfo>> {
        match site.site_type {
            SiteType::SelfHosted => {
                // JOIN with self_hosted_sites
            }
            SiteType::WordPressCom => {
                // JOIN with wordpress_com_sites
            }
        }
    }
}
```

**Key benefit:** Repository APIs don't change when site types are added.

### 4. Zero-Cost Abstraction

**`DbSite` is `Copy` (all primitives):**

```rust
impl Copy for DbSite {}
impl Clone for DbSite {}

// Passing &DbSite has no overhead
let site = DbSite { row_id: RowId(1) };
repo.upsert(&conn, &post, &site)?;  // &DbSite is just a pointer
```

**Memory layout:**

```rust
std::mem::size_of::<DbSite>();  // Returns: 8 bytes (one i64)

// Even with future enhancements, still small
pub struct DbSite {
    pub row_id: RowId,           // 8 bytes
    pub site_type: SiteType,     // 1 byte (enum)
    pub mapped_site_id: RowId,   // 8 bytes
}
// Total: ~16 bytes, still Copy-eligible
```

## Database Schema

### Sites Table

```sql
CREATE TABLE `sites` (
  `id` INTEGER PRIMARY KEY AUTOINCREMENT
) STRICT;
```

**Current implementation:**
- Minimal - just an ID
- Foundation for foreign keys

**Future enhancement:**
- Add `site_type` column
- Create site type tables (self_hosted_sites, wordpress_com_sites)

### Posts Table with FK

```sql
CREATE TABLE `posts_edit_context` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `db_site_id` INTEGER NOT NULL,
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

## Example Usage

### Creating a Site

```rust
// Site repository (future implementation)
pub struct SiteRepository;

impl SiteRepository {
    pub fn get_or_create(
        &self,
        executor: &impl QueryExecutor,
        site_info: &SiteInfo,
    ) -> Result<DbSite> {
        // Check if site exists
        if let Some(site) = self.find_by_domain(executor, &site_info.domain)? {
            return Ok(site);
        }

        // Create new site
        let rowid = executor.execute(
            "INSERT INTO sites DEFAULT VALUES",
            []
        )?;
        Ok(DbSite {
            row_id: executor.last_insert_rowid(),
        })
    }
}
```

### Using Site in Queries

```rust
// Get site reference
let site = site_repo.get_or_create(&conn, &site_info)?;

// All operations scoped to this site
let posts = post_repo.select_all(&conn, &site)?;
let post = post_repo.select_by_post_id(&conn, &site, PostId(123))?;
post_repo.upsert(&conn, &new_post, &site)?;

// Cannot accidentally use wrong site
let other_site = site_repo.get_or_create(&conn, &other_site_info)?;
let other_posts = post_repo.select_all(&conn, &other_site)?;
// Completely isolated from `site`
```

## Alternatives Considered

### Alternative 1: Site ID as i64

```rust
fn select_all(site_id: i64) -> Result<Vec<Post>> { }
```

**Why rejected:**
- ❌ No type safety
- ❌ Confusing with WordPress IDs
- ❌ Doesn't force validation
- ❌ Can't extend with metadata

### Alternative 2: Site Domain as String

```rust
fn select_all(domain: &str) -> Result<Vec<Post>> { }
```

**Why rejected:**
- ❌ Self-hosted sites might change domains
- ❌ String comparison overhead
- ❌ Domain not unique across site types
- ❌ No referential integrity

### Alternative 3: No Multi-Site Support

```rust
// Single site only
fn select_all() -> Result<Vec<Post>> { }
```

**Why rejected:**
- ❌ WordPress apps manage multiple sites
- ❌ Would need major refactor to add later
- ❌ Post ID collisions inevitable

### Alternative 4: Separate Tables Per Site

```sql
CREATE TABLE posts_site_1 (...);
CREATE TABLE posts_site_2 (...);
```

**Why rejected:**
- ❌ Dynamic table creation (SQL injection risk)
- ❌ Cannot query across sites
- ❌ Schema management nightmare
- ❌ Index explosion

## Trade-offs

### Advantages

✅ **Type safety** - Cannot confuse with other IDs
✅ **Data isolation** - Sites completely separated
✅ **Referential integrity** - Foreign keys enforce validity
✅ **Cascade deletion** - Clean site removal
✅ **Query scoping** - Prevents cross-site leaks
✅ **Future-proof** - Can add site metadata without API changes
✅ **Zero-cost** - No runtime overhead

### Disadvantages

❌ **Extra parameter** - Must pass site to every method
❌ **Indirection** - `site.row_id` instead of direct ID
❌ **Requires site lookup** - Must fetch DbSite before queries

**Mitigation:**
- Benefits far outweigh verbosity
- Type safety prevents entire classes of bugs
- Wrapper types can reduce boilerplate if needed

## Related Decisions

- [Type System](../architecture/type-system.md) - `DbSite` definition
- [Database Schema](../architecture/database-schema.md) - Foreign key constraints
- [UPSERT Pattern](06-upsert-pattern.md) - Composite unique index usage

## See Also

- [PostRepository](../repositories/post-repository.md) - Multi-site query examples
- [Usage Examples](../usage-examples.md) - Site scoping patterns
