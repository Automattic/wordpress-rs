# Type System

> **Last Updated:** 2025-10-21

Core type definitions that provide type safety and clarity throughout the caching system.

## Overview

The type system distinguishes between:
- **Database identifiers** - Internal SQLite rowids (`RowId`)
- **Site references** - Multi-site scoping (`DbSite`)
- **Domain entities** - WordPress data models from REST API
- **Database wrappers** - Entities with database metadata attached

## Core Types

### RowId

Internal SQLite row identifier.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowId(pub i64);
```

**Purpose:**
- Type-safe wrapper around SQLite's `rowid`
- Distinguishes from WordPress entity IDs (e.g., `PostId`)
- Used for internal database operations

**Usage:**
```rust
// Returned from insert operations
let rowid: RowId = repo.insert(&conn, &post, &site)?;

// Used in queries
let db_post = repo.select_by_rowid(&conn, &site, rowid)?;
```

**Why Not Just `i64`?**

- **Type safety**: Cannot confuse with WordPress IDs
- **Self-documenting**: Clear intent in function signatures
- **Future-proof**: Can add behavior without changing call sites

### DbSite

Multi-site scope identifier.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DbSite {
    pub row_id: RowId,
}
```

**Purpose:**
- Type-safe reference to a site in the cache
- Required parameter for all repository operations
- Prevents confusion with WordPress.com site IDs

**Usage:**
```rust
// Must fetch from site repository first
let site = site_repo.get_or_create(&conn, &site_info)?;

// Use in all operations
let posts = post_repo.select_all(&conn, &site)?;
let rowid = post_repo.insert(&conn, &post, &site)?;
```

**Why `DbSite` Instead of Just a Numeric ID?**

1. **Prevents confusion with WordPress identifiers**:
   - Clearly a database-internal type (hence `Db` prefix)
   - Cannot be confused with WordPress.com site IDs
   - Self-hosted sites don't have numeric IDs

2. **Forces valid site references**:
   - Callers must fetch valid `DbSite` from site repository
   - Prevents arbitrary construction like `DbSite { row_id: RowId(999) }`
   - Ensures site exists before querying its data

3. **Future-proof for site polymorphism**:
   ```rust
   // Future enhancement
   pub struct DbSite {
       pub row_id: RowId,
       pub site_type: SiteType,      // SelfHosted | WordPressCom
       pub mapped_site_id: RowId,    // FK to specific site type table
   }
   ```

4. **Zero-cost abstraction**:
   - `DbSite` is `Copy` (all primitives)
   - `&DbSite` has no runtime overhead

See [Multi-Site with DbSite](../design-decisions/07-multi-site-dbsite.md) for full rationale.

## Entity Types

### Domain Entities

WordPress data models from the REST API, without database metadata.

**Example:**
```rust
// From wp_api crate
pub struct AnyPostWithEditContext {
    pub id: PostId,  // WordPress post ID
    pub title: PostTitleWithEditContext,
    pub content: PostContentWithEditContext,
    pub author: UserId,
    pub categories: Option<Vec<TermId>>,
    pub tags: Option<Vec<TermId>>,
    // ... other WordPress fields
}
```

**Characteristics:**
- Represents WordPress REST API response structure
- No database-specific fields (no rowid, no site reference)
- Used as input to insert/update operations
- Implements `DbEntity` trait for persistence

**Related:**
- [Core Traits](core-traits.md) - `DbEntity` trait

### Database Wrapper Types

Entities with database metadata (rowid, site, timestamps).

**Example:**
```rust
pub struct DbAnyPostWithEditContext {
    pub row_id: RowId,                     // SQLite rowid
    pub site: DbSite,                       // Site reference
    pub post: AnyPostWithEditContext,      // Domain entity
    pub last_fetched_at: String,           // Cache timestamp
}
```

**Characteristics:**
- Wraps domain entity with database metadata
- Returned from query operations
- Provides access to internal database identifiers
- Implements `TryFromDbRow` for deserialization

**Usage:**
```rust
// Query returns wrapper type
let db_post: DbAnyPostWithEditContext = repo.select_by_rowid(&conn, &site, rowid)?;

// Access database metadata
println!("RowID: {:?}", db_post.row_id);
println!("Site: {:?}", db_post.site);
println!("Cached: {}", db_post.last_fetched_at);

// Access domain entity
println!("Title: {}", db_post.post.title.raw);
```

**Why Separate Types?**

See [Entity vs Wrapper Types](../design-decisions/05-entity-vs-wrapper.md) for complete rationale.

**Key Benefits:**
- **Clean domain model**: WordPress entities remain focused on API structure
- **Type safety**: Compiler ensures proper handling of database metadata
- **Flexibility**: Can add database fields without changing domain types

## Term Relationship Types

### DbTermRelationship

Normalized term association.

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

**Purpose:**
- Represents a single term-to-object association
- Links posts/pages to categories/tags
- Supports custom taxonomies

**Field Details:**
- `object_id` - rowid from `posts_edit_context` (or other entity table)
- `term_id` - WordPress term ID from REST API
- `taxonomy_type` - Type of taxonomy (uses `wp_api` enum)

**Related:**
- [Term Normalization](../design-decisions/09-term-normalization.md)
- [TermRelationshipRepository](../repositories/term-relationship-repository.md)

## Type Conversions

### WordPress IDs vs RowIds

```rust
// WordPress entity IDs (from wp_api crate)
pub struct PostId(pub i64);
pub struct UserId(pub i64);
pub struct TermId(pub i64);

// Database rowid (internal)
pub struct RowId(pub i64);
```

**Important Distinction:**
- `PostId`, `UserId`, etc. are WordPress identifiers (from REST API)
- `RowId` is SQLite's internal identifier
- Never confuse the two - they represent different namespaces

**Example:**
```rust
// A post has BOTH a WordPress ID and a database rowid
let wordpress_id: PostId = PostId(123);  // From REST API
let db_rowid: RowId = RowId(456);         // From SQLite

// Query by WordPress ID returns wrapper with both
let db_post = repo.select_by_post_id(&conn, &site, wordpress_id)?;
assert_eq!(db_post.post.id, PostId(123));  // WordPress ID
assert_eq!(db_post.row_id, RowId(456));     // Database rowid
```

### Timestamp Format

```rust
// ISO 8601 UTC with milliseconds
pub type Timestamp = String;  // "2025-10-21T19:49:22.667Z"
```

**Usage:**
```rust
let db_post = repo.select_by_rowid(&conn, &site, rowid)?;
println!("Last fetched: {}", db_post.last_fetched_at);
// Output: "2025-10-21T19:49:22.667Z"
```

**Parsing:**
```rust
use chrono::{DateTime, Utc};

let dt = DateTime::parse_from_rfc3339(&db_post.last_fetched_at)?;
let age = Utc::now() - dt;
```

See [Cache Freshness](../design-decisions/10-cache-freshness.md) for timestamp handling.

## Trait Implementations

### Copy Types

Zero-cost types implementing `Copy`:

```rust
impl Copy for RowId {}
impl Copy for DbSite {}
```

**Benefits:**
- Pass by value without ownership transfer
- No lifetime complexity
- Compiler optimizes to zero overhead

### Debug/Display

All core types implement standard traits:

```rust
impl Debug for RowId { /* ... */ }
impl Display for RowId { /* ... */ }

impl Debug for DbSite { /* ... */ }
impl Display for DbSite { /* ... */ }
```

**Usage:**
```rust
println!("{:?}", rowid);  // Debug output
println!("{}", site);     // Display output
```

## Type Safety Examples

### Compile-Time Protection

```rust
// ❌ Won't compile - wrong ID type
let post_id = PostId(123);
repo.select_by_rowid(&conn, &site, post_id)?;  // Type error!

// ✅ Correct
let rowid = RowId(456);
repo.select_by_rowid(&conn, &site, rowid)?;
```

### Self-Documenting Code

```rust
// Clear intent from function signature
pub fn select_by_post_id(
    &self,
    executor: &impl QueryExecutor,
    site: &DbSite,      // Must provide site scope
    post_id: PostId,     // WordPress ID, not rowid
) -> Result<DbAnyPostWithEditContext, SqliteDbError>
```

### Forced Validation

```rust
// Cannot create arbitrary DbSite - must fetch from repository
let site = DbSite { row_id: RowId(999) };  // ❌ Bad practice - site may not exist

// ✅ Correct approach
let site = site_repo.get_or_create(&conn, &site_info)?;  // Ensures validity
```

## Future Type Evolution

### Planned Enhancements

1. **Site type polymorphism**:
   ```rust
   pub enum SiteType {
       SelfHosted,
       WordPressCom,
   }

   pub struct DbSite {
       pub row_id: RowId,
       pub site_type: SiteType,
       pub mapped_site_id: RowId,  // FK to type-specific table
   }
   ```

2. **Generic wrapper type**:
   ```rust
   pub struct DbEntity<T> {
       pub row_id: RowId,
       pub site: DbSite,
       pub entity: T,
       pub last_fetched_at: String,
   }

   pub type DbPost = DbEntity<AnyPostWithEditContext>;
   pub type DbPage = DbEntity<PageWithEditContext>;
   ```

3. **Typed taxonomy wrapper**:
   ```rust
   pub struct TypedTaxonomy {
       pub taxonomy_type: TaxonomyType,
       pub terms: Vec<TermId>,
   }
   ```

## See Also

- [Core Traits](core-traits.md) - `DbEntity`, `Repository` traits
- [Database Schema](database-schema.md) - Table structure for these types
- [Entity vs Wrapper Types](../design-decisions/05-entity-vs-wrapper.md) - Design rationale
- [Multi-Site with DbSite](../design-decisions/07-multi-site-dbsite.md) - DbSite design rationale
