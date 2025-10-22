# Database Schema

> **Last Updated:** 2025-10-21

Complete database schema definitions, indexes, and migration strategy for the wp_mobile_cache SQLite database.

## Overview

The database uses SQLite's STRICT mode for type safety and implements multi-site architecture through foreign key relationships. All entity tables reference a central `sites` table.

## Core Tables

### sites

Foundation table for multi-site architecture.

```sql
CREATE TABLE `sites` (
  `id` INTEGER PRIMARY KEY AUTOINCREMENT
) STRICT;
```

**Purpose:**
- Central registry of all cached WordPress sites
- Foundation for foreign key relationships
- Enables cascade deletion of site data

**Related Decisions:**
- [Multi-Site with DbSite](../design-decisions/07-multi-site-dbsite.md)

### posts_edit_context

Stores WordPress posts in edit context (full field set).

```sql
CREATE TABLE `posts_edit_context` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `db_site_id` INTEGER NOT NULL,
  `id` INTEGER NOT NULL,  -- WordPress post ID
  `last_fetched_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  -- ... post fields ...

  FOREIGN KEY (db_site_id) REFERENCES sites(id) ON DELETE CASCADE
) STRICT;
```

**Key Fields:**
- `rowid` - SQLite internal row identifier
- `db_site_id` - Foreign key to `sites` table
- `id` - WordPress post ID (from REST API)
- `last_fetched_at` - Cache timestamp (ISO 8601 UTC)

**Constraints:**
- Foreign key to `sites` with cascade deletion
- Composite unique index on `(db_site_id, id)`

**Indexes:**

```sql
-- Unique constraint on WordPress post ID per site
CREATE UNIQUE INDEX idx_posts_edit_context_unique_db_site_id_and_id
  ON posts_edit_context(db_site_id, id);

-- Query performance for site-scoped queries
CREATE INDEX idx_posts_edit_context_db_site_id
  ON posts_edit_context(db_site_id);
```

**Related Decisions:**
- [UPSERT Pattern](../design-decisions/06-upsert-pattern.md) - Uses composite unique index
- [Cache Freshness](../design-decisions/10-cache-freshness.md) - `last_fetched_at` field

### term_relationships

Normalized storage of term associations (categories, tags, custom taxonomies).

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

**Key Fields:**
- `object_id` - rowid of the associated object (post, page, etc.)
- `term_id` - WordPress term ID from REST API
- `taxonomy_type` - Type of taxonomy (category, post_tag, custom)

**Why No FK on `object_id`?**

The `object_id` references different tables (posts, pages, nav items), so we cannot create a single foreign key constraint. Data integrity is maintained through application logic.

**Indexes:**

```sql
-- Prevent duplicate associations
CREATE UNIQUE INDEX idx_term_relationships_unique
  ON term_relationships(db_site_id, object_id, term_id, taxonomy_type);

-- Query: "Find all objects with taxonomy X and term Y"
CREATE INDEX idx_term_relationships_by_term
  ON term_relationships(db_site_id, taxonomy_type, term_id);

-- Query: "Find all terms for object X" (used in joins when reading posts)
CREATE INDEX idx_term_relationships_by_object
  ON term_relationships(db_site_id, object_id);
```

**Related Decisions:**
- [Term Normalization](../design-decisions/09-term-normalization.md) - Why normalized vs JSON

## Migration Strategy

### File Organization

```
migrations/
├── 0001-create-sites-table.sql           # Foundation
├── 0002-create-posts-table.sql           # Posts with FK to sites
├── 0003-create-term-relationships.sql    # Term associations
└── ...                                    # Future migrations
```

### Migration Order

1. **Sites table first** - Required by all other tables
2. **Entity tables** - Posts, pages, users, etc. with FKs to sites
3. **Relationship tables** - Term relationships, etc.

### Example Migration

```sql
-- migrations/0001-create-sites-table.sql
CREATE TABLE IF NOT EXISTS `sites` (
  `id` INTEGER PRIMARY KEY AUTOINCREMENT
) STRICT;
```

```sql
-- migrations/0002-create-posts-table.sql
CREATE TABLE IF NOT EXISTS `posts_edit_context` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `db_site_id` INTEGER NOT NULL,
  `id` INTEGER NOT NULL,
  `date` TEXT,
  `date_gmt` TEXT,
  `guid_raw` TEXT,
  `modified` TEXT,
  `modified_gmt` TEXT,
  `password` TEXT,
  `slug` TEXT,
  `status` TEXT,
  `type` TEXT,
  `link` TEXT,
  `title_raw` TEXT,
  `content_raw` TEXT,
  `excerpt_raw` TEXT,
  `author` INTEGER,
  `featured_media` INTEGER,
  `comment_status` TEXT,
  `ping_status` TEXT,
  `sticky` INTEGER,
  `template` TEXT,
  `format` TEXT,
  `meta` TEXT,
  `categories` TEXT,
  `tags` TEXT,
  `permalink_template` TEXT,
  `generated_slug` TEXT,
  `last_fetched_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

  FOREIGN KEY (db_site_id) REFERENCES sites(id) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_posts_edit_context_unique_db_site_id_and_id
  ON posts_edit_context(db_site_id, id);

CREATE INDEX IF NOT EXISTS idx_posts_edit_context_db_site_id
  ON posts_edit_context(db_site_id);
```

## Schema Conventions

### Naming

- **Table names**: Lowercase with underscores (e.g., `posts_edit_context`)
- **Foreign keys**: Prefixed with `db_` to distinguish from WordPress IDs (e.g., `db_site_id`)
- **Index names**: Format `idx_{table}_{purpose}` (e.g., `idx_posts_edit_context_db_site_id`)

### Field Types

- **IDs**: `INTEGER` for all numeric identifiers
- **Timestamps**: `TEXT` in ISO 8601 UTC format (`YYYY-MM-DDTHH:MM:SS.fffZ`)
- **Booleans**: `INTEGER` (0 or 1)
- **JSON**: `TEXT` for complex nested data

### Constraints

- **Primary keys**: Always use `AUTOINCREMENT` for consistent behavior
- **Foreign keys**: Always include `ON DELETE CASCADE` for site relationships
- **Unique constraints**: Use composite indexes for natural keys

## STRICT Mode

All tables use SQLite's STRICT mode for type safety:

```sql
CREATE TABLE example (...) STRICT;
```

**Benefits:**
- Enforces column types at insert/update
- Prevents implicit type conversions
- Catches data type errors early

**Trade-offs:**
- More restrictive than standard SQLite
- Requires explicit type declarations

## Timestamp Handling

### Automatic Timestamps

The `last_fetched_at` field uses SQLite's automatic timestamp:

```sql
`last_fetched_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
```

**Behavior:**
- **INSERT**: Automatically set to current UTC time
- **UPDATE**: Must be explicitly set in ON CONFLICT clause

```sql
ON CONFLICT(db_site_id, id) DO UPDATE SET
  -- ... other fields ...
  last_fetched_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
```

**Format:** ISO 8601 with milliseconds: `2025-10-21T19:49:22.667Z`

See [Cache Freshness](../design-decisions/10-cache-freshness.md) for rationale.

## Foreign Key Cascade Deletion

All entity tables use cascade deletion:

```sql
FOREIGN KEY (db_site_id) REFERENCES sites(id) ON DELETE CASCADE
```

**Effect:**
```rust
// Deleting a site cascades to all related entities
execute("DELETE FROM sites WHERE id = ?", params![site_id])?;
// Automatically deletes:
// - All posts for the site
// - All term relationships for those posts
// - Any other entities with FK to sites
```

**Benefits:**
- Automatic cleanup of site data
- Referential integrity maintained
- No orphaned records

## Index Strategy

### Unique Indexes

Used for natural keys (composite business identifiers):

```sql
CREATE UNIQUE INDEX idx_posts_edit_context_unique_db_site_id_and_id
  ON posts_edit_context(db_site_id, id);
```

**Purpose:**
- Enforce uniqueness of WordPress entity per site
- Enable UPSERT operations via ON CONFLICT
- Prevent duplicate data

### Query Indexes

Used for common query patterns:

```sql
CREATE INDEX idx_posts_edit_context_db_site_id
  ON posts_edit_context(db_site_id);
```

**Purpose:**
- Speed up site-scoped queries
- Optimize JOIN operations
- Support WHERE clause filtering

### Composite Indexes

Used for multi-column queries:

```sql
CREATE INDEX idx_term_relationships_by_term
  ON term_relationships(db_site_id, taxonomy_type, term_id);
```

**Purpose:**
- Support queries filtering by multiple columns
- Optimize specific query patterns
- Reduce query planning overhead

## Future Schema Evolution

### Planned Additions

1. **Site type tables**:
   ```sql
   CREATE TABLE self_hosted_sites (
     id INTEGER PRIMARY KEY,
     domain TEXT NOT NULL,
     FOREIGN KEY (id) REFERENCES sites(id)
   );

   CREATE TABLE wordpress_com_sites (
     id INTEGER PRIMARY KEY,
     site_id INTEGER NOT NULL,  -- WordPress.com site ID
     FOREIGN KEY (id) REFERENCES sites(id)
   );
   ```

2. **Additional entity tables**:
   - `pages_edit_context`
   - `users`
   - `media`
   - `categories`
   - `tags`

3. **Cache metadata**:
   - `cache_operations` - Operation history
   - `sync_state` - Sync status per site

See [Migration Guide](../migration-guide.md) for adding new entities.

## See Also

- [Type System](type-system.md) - `RowId`, `DbSite` type definitions
- [Core Traits](core-traits.md) - `DbEntity`, `Repository` traits
- [PostRepository](../repositories/post-repository.md) - Example repository using this schema
- [TermRelationshipRepository](../repositories/term-relationship-repository.md) - Term relationship management
