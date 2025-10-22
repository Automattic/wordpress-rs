# Migration Guide

> **Last Updated:** 2025-10-22

Guide for adding new WordPress entities to the wp_mobile_cache system.

## Overview

This guide walks through adding a new entity type (e.g., pages, users, media) to the cache system. The process follows established patterns from the posts implementation.

## Prerequisites

Before adding a new entity:

1. **Understand existing patterns** - Review `PostRepository` and related code
2. **Have entity type defined** - Entity should exist in `wp_api` crate
3. **Know entity structure** - Understand WordPress REST API fields
4. **Identify relationships** - Does entity have terms or other associations?

## Step-by-Step Guide

### Phase 1: Database Schema

#### 1. Create Migration File

Create `migrations/000X-create-{entity}-table.sql`:

```sql
-- migrations/0004-create-pages-table.sql
CREATE TABLE IF NOT EXISTS `pages_edit_context` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `db_site_id` INTEGER NOT NULL,
  `id` INTEGER NOT NULL,  -- WordPress page ID
  `last_fetched_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

  -- WordPress fields (from wp_api::pages::PageWithEditContext)
  `date` TEXT,
  `modified` TEXT,
  `slug` TEXT,
  `status` TEXT,
  `type` TEXT,
  `title_raw` TEXT,
  `content_raw` TEXT,
  `author` INTEGER,
  `parent` INTEGER,
  `menu_order` INTEGER,
  -- ... add all fields from entity type

  FOREIGN KEY (db_site_id) REFERENCES sites(id) ON DELETE CASCADE
) STRICT;

-- Composite unique index (site + WordPress ID)
CREATE UNIQUE INDEX IF NOT EXISTS idx_pages_edit_context_unique_db_site_id_and_id
  ON pages_edit_context(db_site_id, id);

-- Site scoping index
CREATE INDEX IF NOT EXISTS idx_pages_edit_context_db_site_id
  ON pages_edit_context(db_site_id);
```

**Key points:**
- Table name: `{entity}_edit_context` (matches context level from API)
- Always include: `rowid`, `db_site_id`, `id`, `last_fetched_at`
- Add all entity fields from `wp_api` crate
- Composite unique index on `(db_site_id, id)`
- Site scoping index on `db_site_id`
- Foreign key with `ON DELETE CASCADE`

#### 2. Add to Migration Manager

Update migration counter in `lib.rs` or migration management code.

### Phase 2: Type Definitions

#### 1. Create Wrapper Type

In `src/{entity}.rs`:

```rust
use crate::{RowId, DbSite};
use wp_api::pages::PageWithEditContext;

/// Database wrapper for PageWithEditContext
#[derive(Debug, Clone)]
pub struct DbPageWithEditContext {
    pub row_id: RowId,
    pub site: DbSite,
    pub page: PageWithEditContext,
    pub last_fetched_at: String,
}
```

#### 2. Implement DbEntity Trait

```rust
use crate::repository::DbEntity;

impl DbEntity for PageWithEditContext {
    const TABLE_NAME: &'static str = "pages_edit_context";
}
```

### Phase 3: Database Mappings

#### 1. Implement TryFromDbRow

```rust
use crate::mappings::TryFromDbRow;

impl TryFromDbRow for DbPageWithEditContext {
    fn try_from_db_row(row: &rusqlite::Row) -> Result<Self, SqliteDbError> {
        Ok(Self {
            row_id: RowId(row.get("rowid")?),
            site: DbSite { row_id: RowId(row.get("db_site_id")?) },
            page: PageWithEditContext {
                id: PageId(row.get("id")?),
                date: row.get("date")?,
                modified: row.get("modified")?,
                slug: row.get("slug")?,
                status: row.get("status")?,
                type_: row.get("type")?,
                title: PageTitleWithEditContext {
                    raw: row.get("title_raw")?,
                },
                content: PageContentWithEditContext {
                    raw: row.get("content_raw")?,
                },
                author: UserId(row.get("author")?),
                parent: row.get::<_, Option<i64>>("parent")?.map(PageId),
                menu_order: row.get("menu_order")?,
                // ... map all fields
            },
            last_fetched_at: row.get("last_fetched_at")?,
        })
    }
}
```

### Phase 4: Repository Implementation

#### 1. Create Repository Struct

In `src/repository/{entity}.rs`:

```rust
use crate::{DbSite, RowId};
use crate::repository::{QueryExecutor, TransactionManager};
use wp_api::pages::{PageWithEditContext, PageId};

pub struct PageRepository;
```

#### 2. Implement Common Query Methods

```rust
impl PageRepository {
    pub fn select_by_rowid(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        rowid: RowId,
    ) -> Result<DbPageWithEditContext, SqliteDbError> {
        let sql = "SELECT * FROM pages_edit_context WHERE db_site_id = ? AND rowid = ?";
        let mut stmt = executor.prepare(sql)?;
        stmt.query_row(
            rusqlite::params![site.row_id.0, rowid.0],
            |row| DbPageWithEditContext::try_from_db_row(row)
        )
    }

    pub fn select_all(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
    ) -> Result<Vec<DbPageWithEditContext>, SqliteDbError> {
        let sql = "SELECT * FROM pages_edit_context WHERE db_site_id = ?";
        let mut stmt = executor.prepare(sql)?;
        let pages = stmt.query_map(
            rusqlite::params![site.row_id.0],
            |row| DbPageWithEditContext::try_from_db_row(row)
        )?;
        pages.collect()
    }

    pub fn select_by_page_id(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        page_id: PageId,
    ) -> Result<DbPageWithEditContext, SqliteDbError> {
        let sql = "SELECT * FROM pages_edit_context WHERE db_site_id = ? AND id = ?";
        let mut stmt = executor.prepare(sql)?;
        stmt.query_row(
            rusqlite::params![site.row_id.0, page_id.0],
            |row| DbPageWithEditContext::try_from_db_row(row)
        )
    }
}
```

#### 3. Implement Entity-Specific Methods

```rust
impl PageRepository {
    // Pages-specific: query by parent
    pub fn select_by_parent(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        parent_id: PageId,
    ) -> Result<Vec<DbPageWithEditContext>, SqliteDbError> {
        let sql = "SELECT * FROM pages_edit_context WHERE db_site_id = ? AND parent = ?";
        let mut stmt = executor.prepare(sql)?;
        let pages = stmt.query_map(
            rusqlite::params![site.row_id.0, parent_id.0],
            |row| DbPageWithEditContext::try_from_db_row(row)
        )?;
        pages.collect()
    }

    // Pages-specific: query top-level pages (no parent)
    pub fn select_top_level(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
    ) -> Result<Vec<DbPageWithEditContext>, SqliteDbError> {
        let sql = "SELECT * FROM pages_edit_context WHERE db_site_id = ? AND parent IS NULL ORDER BY menu_order";
        let mut stmt = executor.prepare(sql)?;
        let pages = stmt.query_map(
            rusqlite::params![site.row_id.0],
            |row| DbPageWithEditContext::try_from_db_row(row)
        )?;
        pages.collect()
    }
}
```

#### 4. Implement UPSERT

If the entity does NOT have term relationships, implement a simple upsert:

```rust
impl PageRepository {
    pub fn upsert(
        &self,
        transaction_manager: &mut impl TransactionManager,
        site: &DbSite,
        page: &PageWithEditContext,
    ) -> Result<RowId, SqliteDbError> {
        let tx = transaction_manager.transaction()?;

        tx.execute(
            r#"
            INSERT INTO pages_edit_context (
                db_site_id, id, date, modified, slug, status, type,
                title_raw, content_raw, author, parent, menu_order,
                last_fetched_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            ON CONFLICT(db_site_id, id) DO UPDATE SET
                date = excluded.date,
                modified = excluded.modified,
                slug = excluded.slug,
                status = excluded.status,
                type = excluded.type,
                title_raw = excluded.title_raw,
                content_raw = excluded.content_raw,
                author = excluded.author,
                parent = excluded.parent,
                menu_order = excluded.menu_order,
                last_fetched_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            "#,
            rusqlite::params![
                site.row_id.0,
                page.id.0,
                page.date,
                page.modified,
                page.slug,
                page.status,
                page.type_,
                page.title.raw,
                page.content.raw,
                page.author.0,
                page.parent.map(|p| p.0),
                page.menu_order,
            ],
        )?;

        let rowid = tx.last_insert_rowid();
        tx.commit()?;
        Ok(rowid)
    }
}
```

**Note:** If the entity has term relationships (categories, tags, etc.), see "Adding Entity with Terms" section below for the full implementation.

#### 5. Implement Delete and Count

```rust
impl PageRepository {
    pub fn delete_by_page_id(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        page_id: PageId,
    ) -> Result<usize, SqliteDbError> {
        executor.execute(
            "DELETE FROM pages_edit_context WHERE db_site_id = ? AND id = ?",
            rusqlite::params![site.row_id.0, page_id.0],
        )
    }

    pub fn count(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
    ) -> Result<i64, SqliteDbError> {
        let sql = "SELECT COUNT(*) FROM pages_edit_context WHERE db_site_id = ?";
        let mut stmt = executor.prepare(sql)?;
        stmt.query_row(rusqlite::params![site.row_id.0], |row| row.get(0))
    }
}
```

### Phase 5: Module Organization

#### Update `src/lib.rs`

```rust
// Type definitions
pub mod pages;
pub use pages::DbPageWithEditContext;

// Repository
pub mod repository {
    mod pages;
    pub use pages::PageRepository;
}

// Mappings
mod mappings {
    mod pages;
}
```

### Phase 6: Testing

#### 1. Unit Tests

In `src/repository/{entity}.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_page_upsert_and_select() {
        let mut conn = Connection::open_in_memory().unwrap();
        // Run migrations...

        let repo = PageRepository;
        let site = DbSite { row_id: RowId(1) };

        let page = create_test_page();
        let rowid = repo.upsert(&mut conn, &site, &page).unwrap();

        let db_page = repo.select_by_rowid(&conn, &site, rowid).unwrap();
        assert_eq!(db_page.page.id, page.id);
    }

    #[test]
    fn test_page_upsert() {
        // Test insert and update scenarios
    }

    #[test]
    fn test_select_by_parent() {
        // Test hierarchical queries
    }
}
```

#### 2. Integration Tests

Create `tests/test_{entity}.rs`:

```rust
use wp_mobile_cache::{WpApiCache, DbSite, RowId};
use wp_mobile_cache::repository::PageRepository;

#[test]
fn test_page_crud() {
    let cache = WpApiCache::new(":memory:").unwrap();
    cache.perform_migrations().unwrap();
    let conn = cache.connection();

    // Create site
    conn.execute("INSERT INTO sites DEFAULT VALUES", []).unwrap();
    let site = DbSite { row_id: RowId(1) };

    let repo = PageRepository;

    // Test insert, select, update, delete
}
```

## Adding Entity with Terms

If the entity has term relationships (like posts):

### 1. Add Term Support to Entity

The entity type should have term fields:

```rust
pub struct PageWithEditContext {
    pub id: PageId,
    pub categories: Option<Vec<TermId>>,  // If pages support categories
    // ...
}
```

### 2. Extend upsert() to Handle Terms

Modify your `upsert()` implementation to sync term relationships automatically:

```rust
impl PageRepository {
    pub fn upsert(
        &self,
        transaction_manager: &mut impl TransactionManager,
        site: &DbSite,
        page: &PageWithEditContext,
    ) -> Result<RowId, SqliteDbError> {
        let tx = transaction_manager.transaction()?;

        // Upsert the page
        tx.execute(
            r#"
            INSERT INTO pages_edit_context (...)
            VALUES (...)
            ON CONFLICT(db_site_id, id) DO UPDATE SET ...
            "#,
            rusqlite::params![/* params */],
        )?;

        let page_rowid = tx.last_insert_rowid();

        // Sync term relationships
        let term_repo = TermRelationshipRepository;

        if let Some(ref categories) = page.categories {
            term_repo.sync_terms_for_object(
                &tx, site, page_rowid, &TaxonomyType::Category, categories
            )?;
        }

        tx.commit()?;
        Ok(page_rowid)
    }
}
```

### 3. Update Reads to Include Terms

```rust
impl PageRepository {
    pub fn select_by_rowid(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        rowid: RowId,
    ) -> Result<DbPageWithEditContext, SqliteDbError> {
        // Query page
        let mut page = /* ... */;

        // Get terms
        let term_repo = TermRelationshipRepository;
        let terms_map = term_repo.get_all_terms_for_object(executor, site, rowid)?;

        // Populate term fields
        page.categories = terms_map.get(&TaxonomyType::Category).cloned();

        Ok(DbPageWithEditContext {
            row_id: rowid,
            site: *site,
            page,
            last_fetched_at,
        })
    }
}
```

### 4. Update Delete to Remove Terms

```rust
impl PageRepository {
    pub fn delete_by_page_id(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        page_id: PageId,
    ) -> Result<usize, SqliteDbError> {
        // Get rowid
        let db_page = self.select_by_page_id(executor, site, page_id)?;

        // Delete term relationships
        let term_repo = TermRelationshipRepository;
        term_repo.delete_all_terms_for_object(executor, site, db_page.row_id)?;

        // Delete page
        executor.execute(
            "DELETE FROM pages_edit_context WHERE db_site_id = ? AND id = ?",
            rusqlite::params![site.row_id.0, page_id.0],
        )
    }
}
```

## Common Patterns

### Handling Optional Fields

```rust
// In SQL INSERT/UPDATE
self.optional_field,  // Option<T> works directly

// In rusqlite params
row.get::<_, Option<i64>>("optional_field")?.map(SomeId),
```

### Handling Nested Structures

```rust
// If entity has nested types that need JSON serialization
use serde_json;

// Insert
":nested_field": serde_json::to_string(&self.nested_field)?,

// Query
let nested_json: String = row.get("nested_field")?;
let nested_field = serde_json::from_str(&nested_json)?;
```

### Handling Lists/Arrays

For simple ID lists, consider:
- **Option 1:** Use `term_relationships` pattern (normalized table)
- **Option 2:** JSON array (if queryability not needed)

```rust
// JSON array approach
":categories": serde_json::to_string(&self.categories)?,
```

## Checklist

When adding a new entity, ensure:

- [ ] Migration file created with proper schema
- [ ] Wrapper type defined (`Db{Entity}`)
- [ ] `DbEntity` trait implemented
- [ ] `TryFromDbRow` trait implemented
- [ ] Repository struct created (zero-sized)
- [ ] Query methods implemented (select_by_rowid, select_all, select_by_id)
- [ ] Entity-specific methods implemented
- [ ] UPSERT method implemented
- [ ] Delete method implemented
- [ ] Count method implemented
- [ ] Module exports updated
- [ ] Unit tests added
- [ ] Integration tests added
- [ ] Documentation added

## File Checklist

Files to create/modify:

- [ ] `migrations/000X-create-{entity}-table.sql`
- [ ] `src/{entity}.rs` - Wrapper type
- [ ] `src/mappings/{entity}.rs` - TryFromDbRow
- [ ] `src/repository/{entity}.rs` - Repository implementation
- [ ] `src/lib.rs` - Module exports
- [ ] `tests/test_{entity}.rs` - Integration tests
- [ ] `docs/repositories/{entity}-repository.md` - Documentation

## Reference Implementations

Study these files as examples:

- **Posts:** Most complete reference implementation
  - `src/posts.rs`
  - `src/mappings/posts.rs`
  - `src/repository/posts.rs`
  - `tests/test_posts.rs`

- **Term Relationships:** Normalized relationships pattern
  - `src/term_relationships.rs`
  - `src/mappings/term_relationships.rs`
  - `src/repository/term_relationships.rs`

## Related Documentation

- [PostRepository API](repositories/post-repository.md) - Complete example
- [TermRelationshipRepository API](repositories/term-relationship-repository.md) - Terms pattern
- [Database Schema](architecture/database-schema.md) - Schema conventions
- [Type System](architecture/type-system.md) - Type patterns
- [Core Traits](architecture/core-traits.md) - Trait requirements
- [Usage Examples](usage-examples.md) - Common patterns

## Getting Help

If you run into issues:

1. Check existing repository implementations
2. Review design decision documents
3. Ensure types from `wp_api` crate are correct
4. Verify SQL schema matches type structure
5. Test with in-memory database first
