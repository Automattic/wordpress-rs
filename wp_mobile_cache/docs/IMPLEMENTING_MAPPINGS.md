# Implementing Database Mappings for WordPress REST API Types

This guide explains how to implement database mappings for WordPress REST API contextual types.

## Understanding WordPress REST API Contexts

WordPress REST API returns different fields based on the `context` parameter (edit, view, embed). The specific fields available in each context vary by endpoint and are defined by WordPress core.

**Key points:**
- Contexts determine which fields are returned by the API
- Field availability is defined in the `wp_api` crate using `#[WpContext(...)]` attributes on the `Sparse*` types
- Not all contexts follow a predictable pattern - you must inspect the generated types to see actual fields
- The `wp_api` crate uses procedural macros to generate context-specific types from `Sparse*` types

**Example differences** (for posts):
- Edit context typically includes fields like `password`, `permalink_template`, and raw content
- View context typically excludes sensitive/edit-only fields but includes most public data
- Embed context typically includes only minimal fields for embedding in other content

However, these are generalizations - **always verify by expanding the actual types** (see Step 1).

## Implementing Mappings for a New Entity Type

### Step 1: Generate Concrete Types

The `wp_api` crate uses the `#[WpContextual]` macro to generate context-specific types. To see the actual generated types:

```bash
# Generate expanded code for a module (e.g., posts)
cargo expand -p wp_api posts > /tmp/generated_posts.rs

# Or for comments
cargo expand -p wp_api comments > /tmp/generated_comments.rs
```

### Step 2: Find the Generated Types

Search for the context-specific struct definitions:

```bash
# Find all contextual type definitions
grep -n "pub struct.*With.*Context" /tmp/generated_posts.rs

# Example output:
# 6704:    pub struct AnyPostWithEditContext {
# 11500:   pub struct AnyPostWithEmbedContext {
# 13648:   pub struct AnyPostWithViewContext {
```

Extract the field definitions for each context:

```bash
# View specific type definition (adjust line numbers from grep output)
sed -n '6704,6750p' /tmp/generated_posts.rs
```

**Also check nested types** that may differ by context:

```bash
# Find nested type definitions (e.g., PostGuid, PostTitle, PostContent)
grep -n "pub struct Post.*With.*Context" /tmp/generated_posts.rs
```

### Step 3: Analyze Field Differences

Compare the fields across contexts to understand what needs to be stored:

1. **Identify common fields**: Present in all contexts
2. **Identify context-specific fields**: Only in certain contexts
3. **Note nested type differences**: Fields within nested structs may differ
   - Example: `PostTitleWithEditContext` has `raw` + `rendered`
   - But: `PostTitleWithViewContext` only has `rendered`
4. **Document your findings**: Create a `CONTEXT_FIELD_ANALYSIS.md` file for reference

**Example analysis structure:**

```markdown
## AnyPostWithEditContext (28 fields)
- id: PostId
- date: String
- password: String (EDIT ONLY)
- title: PostTitleWithEditContext (has raw + rendered)
- ...

## AnyPostWithViewContext (25 fields)
- id: PostId
- date: String
- (no password field)
- title: PostTitleWithViewContext (only rendered)
- ...
```

See `CONTEXT_FIELD_ANALYSIS.md` for a complete example.

### Step 4: Create Database Type Definitions

For each context, create type definitions in `src/db_types/{entity}/`:

```
src/db_types/posts/
├── mod.rs      # Re-exports all context types
├── edit.rs     # EditContext types (column enum + wrapper struct)
├── view.rs     # ViewContext types (column enum + wrapper struct)
└── embed.rs    # EmbedContext types (column enum + wrapper struct)
```

Each type definition file contains two components:

#### 4.1 Column Index Enum

Defines the position of each column in SQL SELECT results:

```rust
/// Column indexes for posts_edit_context table.
/// These must match the order of columns in the CREATE TABLE statement.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub enum PostEditContextColumn {
    Rowid = 0,
    SiteId = 1,
    Id = 2,
    Date = 3,
    DateGmt = 4,
    // ... enumerate all columns in order
}

impl ColumnIndex for PostEditContextColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}
```

**Critical**: Column indexes must match the exact order in the `CREATE TABLE` statement and in all SELECT queries.

#### 4.2 Database Entity Wrapper

Wraps the wp_api type with database metadata:

```rust
pub struct DbAnyPostWithEditContext {
    pub row_id: RowId,              // SQLite rowid
    pub site: DbSite,                // Which WordPress site
    pub post: AnyPostWithEditContext, // The actual wp_api type
    pub last_fetched_at: String,     // Cache timestamp
}
```

**Note**: The `src/db_types/{entity}/*.rs` files contain ONLY these type definitions (column enums and wrapper structs). They are kept intentionally minimal.

### Step 5: Implement Row Mapping in Repository

The actual database operations (mapping rows to types, upsert logic) are implemented in `src/repository/{entity}.rs`. This keeps all database interaction logic in one place.

#### 5.1 Implement FromRowWithTerms Trait

In the repository file (e.g., `src/repository/posts.rs`), implement the private `FromRowWithTerms` trait for each context:

```rust
// In src/repository/posts.rs

// Private trait for generic read operations
trait FromRowWithTerms: Sized {
    fn from_row_with_terms(
        row: &Row,
        term_relationships: Vec<DbTermRelationship>,
    ) -> Result<Self, SqliteDbError>;
}

impl FromRowWithTerms for DbAnyPostWithEditContext {
    fn from_row_with_terms(
        row: &Row,
        term_relationships: Vec<DbTermRelationship>,
    ) -> Result<Self, SqliteDbError> {
        use PostEditContextColumn::*;

        // Extract SQLite metadata
        let row_id: RowId = row.get_column(Rowid)?;
        let site = DbSite {
            row_id: row.get_column(SiteId)?,
        };

        // Extract categories/tags from term relationships
        // (Only for entities that support taxonomies like posts)
        let (categories, tags) = term_relationships.into_iter().fold(
            (Vec::new(), Vec::new()),
            |(mut cats, mut tags), relationship| {
                match relationship.taxonomy_type {
                    TaxonomyType::Category => cats.push(relationship.term_id),
                    TaxonomyType::PostTag => tags.push(relationship.term_id),
                    _ => {} // Ignore other taxonomy types
                }
                (cats, tags)
            },
        );

        // Map database row to wp_api type using helper functions
        let post = AnyPostWithEditContext {
            id: get_id(row, Id)?,
            date: row.get_column(Date)?,
            date_gmt: parse_datetime(row, DateGmt)?,
            guid: PostGuidWithEditContext {
                raw: row.get_column(GuidRaw)?,
                rendered: row.get_column(GuidRendered)?,
            },
            // ... map all other fields
            categories: if categories.is_empty() { None } else { Some(categories) },
            tags: if tags.is_empty() { None } else { Some(tags) },
        };

        Ok(Self {
            row_id,
            site,
            post,
            last_fetched_at: row.get_column(LastFetchedAt)?,
        })
    }
}
```

#### 5.2 Helper Functions Available

The `mappings::helpers` module provides utilities for common conversions in your repository implementations:

**ID Extraction:**
- `get_id(row, column)` - Extract ID types (PostId, UserId, TermId, etc.)
- `get_optional_id(row, column)` - Extract optional IDs

**Type Parsing:**
- `parse_datetime(row, column)` - Parse WpGmtDateTime from TEXT
- `parse_enum(row, column)` - Parse enum types (PostStatus, PostFormat, etc.)
- `parse_optional_enum(row, column)` - Parse optional enums

**Value Conversion:**
- `integer_to_bool(value)` - Convert SQLite INTEGER (0/1) to bool
- `deserialize_json_value(value)` - Parse JSON TEXT columns into Rust types
- `serialize_value_to_json(value)` - Serialize Rust types to JSON for storage

**RowExt Trait:**
- `row.get_column(ColumnEnum)` - Type-safe column access using enum

See `src/repository/posts.rs` (specifically the `FromRowWithTerms` implementations) for a complete reference.

### Step 6: Create Database Migrations

Create SQL migration files for each context's table:

```
migrations/
├── 0002-create-posts-edit-context-table.sql
├── 0004-create-posts-view-context-table.sql
└── 0005-create-posts-embed-context-table.sql
```

**Migration template:**

```sql
-- migrations/NNNN-create-{entity}-{context}-context-table.sql

CREATE TABLE `{entity}_{context}_context` (
  -- Internal DB metadata
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `db_site_id` INTEGER NOT NULL REFERENCES sites(id) ON DELETE CASCADE,

  -- Map each field from the generated wp_api type to a column
  -- (see Step 2 for the actual field list)
  `id` INTEGER NOT NULL,
  `date` TEXT NOT NULL,

  -- Nested types are flattened:
  -- PostGuidWithEditContext { raw: Option<String>, rendered: String }
  -- becomes:
  `guid_raw` TEXT,
  `guid_rendered` TEXT NOT NULL,

  -- ... all other fields

  -- Cache metadata
  `last_fetched_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

  FOREIGN KEY (db_site_id) REFERENCES sites(id) ON DELETE CASCADE
) STRICT;

-- Unique constraint on (site, WordPress ID)
CREATE UNIQUE INDEX idx_{entity}_{context}_unique_db_site_id_and_id
  ON {entity}_{context}_context(db_site_id, id);

-- Index for site-based queries
CREATE INDEX idx_{entity}_{context}_db_site_id
  ON {entity}_{context}_context(db_site_id);
```

**Column Type Mapping:**

| Rust Type | SQLite Type | Notes |
|-----------|-------------|-------|
| `String` | `TEXT` | |
| `i64`, `u32`, `PostId`, etc. | `INTEGER` | All ID types map to INTEGER |
| `bool` | `INTEGER` | 0 = false, 1 = true |
| `WpGmtDateTime` | `TEXT` | ISO 8601 format |
| Enums (`PostStatus`, etc.) | `TEXT` | String representation |
| Complex types (`PostMeta`) | `TEXT` | JSON serialized |
| `Option<T>` | Nullable column | Use NULL for None |

**Flattening nested types:**

```rust
// Rust type:
pub struct PostTitleWithEditContext {
    pub raw: Option<String>,
    pub rendered: String,
}

// SQL columns:
`title_raw` TEXT,
`title_rendered` TEXT NOT NULL
```

### Step 7: Update Module Exports

Add the new types to `src/db_types/mod.rs`:

```rust
pub mod posts;
```

The db_types are re-exported in the repository module for the public API:

```rust
// In src/repository/posts.rs
pub use crate::db_types::posts::{
    DbAnyPostWithEditContext as DbPostEdit,
    DbAnyPostWithEmbedContext as DbPostEmbed,
    DbAnyPostWithViewContext as DbPostView,
};
```

### Step 8: Testing

Create unit tests in the repository file (e.g., `src/repository/posts.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{TestContext, test_ctx};
    use rstest::*;

    #[rstest]
    #[case(PostBuilder::minimal().build())]
    #[case(PostBuilder::full().build())]
    fn test_round_trip(
        mut test_ctx: TestContext,
        #[case] original: AnyPostWithEditContext
    ) {
        let rowid = test_ctx.repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &original)
            .unwrap();

        let retrieved = test_ctx.repo
            .select_by_rowid(&test_ctx.conn, &test_ctx.site, rowid)
            .unwrap();

        assert_eq!(retrieved.post, original);
    }

    // Test each enum variant
    #[rstest]
    #[case(PostStatus::Publish)]
    #[case(PostStatus::Draft)]
    fn test_enum_variants(
        mut test_ctx: TestContext,
        #[case] status: PostStatus
    ) {
        let post = PostBuilder::minimal().with_status(status.clone()).build();
        // ... test round-trip
    }
}
```

## Quick Reference Checklist

When implementing mappings for a new entity (e.g., "implement mappings for SparseComment"):

- [ ] Run `cargo expand -p wp_api comments > /tmp/generated_comments.rs`
- [ ] Find context-specific types: `grep "pub struct.*Comment.*Context" /tmp/generated_comments.rs`
- [ ] Analyze field differences between Edit/View/Embed contexts
- [ ] Create `src/db_types/comments/mod.rs` and context files (`edit.rs`, `view.rs`, `embed.rs`)
- [ ] For each context in `src/db_types/comments/*.rs`:
  - [ ] Define column index enum
  - [ ] Define `DbCommentWith{Context}` wrapper struct
- [ ] In `src/repository/comments.rs`:
  - [ ] Implement `FromRowWithTerms` trait for each context
  - [ ] Implement repository CRUD methods (using generic `PostRepository<C>` pattern)
  - [ ] Implement context-specific upsert methods (EditContext only)
- [ ] Create migration files for each context table
- [ ] Update `src/db_types/mod.rs` to export new module
- [ ] Write unit tests for round-trip persistence in repository file
- [ ] Verify `cargo test` passes

## Common Patterns

### Handling Optional Nested Types

```rust
// excerpt is Option<SparsePostExcerpt>
excerpt: {
    let excerpt_rendered: Option<String> = row.get_column(ExcerptRendered)?;
    if excerpt_rendered.is_some() {
        Some(SparsePostExcerpt {
            raw: row.get_column(ExcerptRaw)?,
            rendered: excerpt_rendered,
            protected: row.get_column(ExcerptProtected)?,
        })
    } else {
        None
    }
}
```

### Converting Empty Vecs to None

```rust
categories: if categories.is_empty() {
    None
} else {
    Some(categories)
}
```

### Handling Booleans

```rust
// Storing (in repository upsert):
":sticky": bool_to_integer(post.sticky),

// Reading (in mapping):
sticky: integer_to_bool(row.get_column(Sticky)?),
```

## File Organization

```
wp_mobile_cache/
├── docs/
│   └── IMPLEMENTING_MAPPINGS.md    # This file
├── src/
│   ├── context.rs                  # Context trait definitions (EditContext, ViewContext, etc.)
│   ├── db_types/
│   │   ├── mod.rs                  # Re-exports entity modules
│   │   ├── posts/
│   │   │   ├── mod.rs              # Re-exports all context types
│   │   │   ├── edit.rs             # Column enum + DbAnyPostWithEditContext
│   │   │   ├── view.rs             # Column enum + DbAnyPostWithViewContext
│   │   │   └── embed.rs            # Column enum + DbAnyPostWithEmbedContext
│   │   └── comments/               # Same structure
│   │       ├── mod.rs
│   │       ├── edit.rs
│   │       ├── view.rs
│   │       └── embed.rs
│   ├── mappings/
│   │   ├── mod.rs
│   │   └── helpers.rs              # Shared conversion utilities
│   └── repository/
│       ├── mod.rs
│       ├── posts.rs                # PostRepository<C> + FromRowWithTerms impls + upsert
│       └── comments.rs             # CommentRepository<C> + FromRowWithTerms impls + upsert
└── migrations/
    ├── 0001-create-sites-table.sql
    ├── 0002-create-posts-edit-context-table.sql
    ├── 0003-create-term-relationships.sql
    ├── 0004-create-posts-view-context-table.sql
    └── 0005-create-posts-embed-context-table.sql
```

**Architecture Summary:**
- **`context.rs`**: Trait definitions for context markers (EditContext, ViewContext, EmbedContext) with associated types
- **`db_types/`**: Type-only definitions (column enums, wrapper structs) - no logic
- **`mappings/helpers.rs`**: Reusable conversion utilities (parse_enum, integer_to_bool, etc.)
- **`repository/`**: All database interaction logic including row-to-type mapping AND SQL operations

## Related Documentation

- `CONTEXT_FIELD_ANALYSIS.md` - Detailed field comparison for posts (example)
- `CONTEXT_SUPPORT_DESIGN.md` - Overall architecture design for context support
- `../wp_api/CLAUDE.md` - Information about the wp_api types being cached
