SQLite-based caching layer for WordPress REST API data with multi-site support, using the Repository pattern for clean database abstractions.

## Quick Start

```rust
use wp_mobile_cache::{WpApiCache, DbSite, RowId};
use wp_mobile_cache::repository::{PostRepository, Repository};

// Open cache database
let cache = WpApiCache::new("cache.db")?;
cache.perform_migrations()?;

// Define site
let site = DbSite { row_id: RowId(1) };

// Use repository
let repo = PostRepository;
let posts = repo.select_all(&cache.connection(), &site)?;
```

## Documentation Structure

### 📚 Architecture

Core system design and components:

- **[Core Traits](architecture/core-traits.md)** - `QueryExecutor`, `TransactionManager`, `DbEntity`, `Repository`
- **[Database Schema](architecture/database-schema.md)** - Complete table definitions and indexes
- **[Type System](architecture/type-system.md)** - `RowId`, `DbSite`, wrapper types

### 🎯 Design Decisions

Rationale behind architectural choices:

1. **[Explicit Executor Passing](design-decisions/01-executor-passing.md)** - Why executors are passed explicitly, not stored
2. **[Associated Types](design-decisions/02-associated-types.md)** - Using associated types over generic parameters
3. **[Zero-Sized Repositories](design-decisions/03-zero-sized-repos.md)** - Stateless repository pattern
4. **[Minimal Abstraction](design-decisions/04-minimal-abstraction.md)** - QueryExecutor design strategy
5. **[Entity vs Wrapper Types](design-decisions/05-entity-vs-wrapper.md)** - Domain entities vs database wrappers
6. **[UPSERT Pattern](design-decisions/06-upsert-pattern.md)** - Insert-or-update operations
7. **[Multi-Site with DbSite](design-decisions/07-multi-site-dbsite.md)** - Type-safe site scoping
8. **[Split Traits](design-decisions/08-split-traits.md)** - Separating `QueryExecutor` and `TransactionManager`
9. **[Term Normalization](design-decisions/09-term-normalization.md)** - Normalized term relationships table
10. **[Cache Freshness](design-decisions/10-cache-freshness.md)** - `last_fetched_at` timestamp tracking

### 🔧 Repository APIs

API reference for each repository:

- **[PostRepository](repositories/post-repository.md)** - Full API for post operations
- **[TermRelationshipRepository](repositories/term-relationship-repository.md)** - Term association management

### 📖 Guides

- **[Usage Examples](usage-examples.md)** - Common operations and patterns
- **[Migration Guide](migration-guide.md)** - Adding new entities and tables

## Goals

1. **Reduce boilerplate** - Eliminate repetitive database code
2. **Type safety** - Strong typing for all database entities
3. **Flexibility** - Custom methods per entity type
4. **Database abstraction** - Decouple from `rusqlite` implementation
5. **Simplicity** - Keep design explicit, avoid over-engineering
6. **Multi-site support** - All operations scoped to specific site

## Non-Goals

1. **Complex wrappers** - No wrapper types holding connections
2. **ORM features** - No lazy loading, change tracking, etc.
3. **Query builders** - No complex query building DSLs

## Key Concepts

### Multi-Site Architecture

All data is scoped to a `DbSite` identifier:

```rust
pub struct DbSite {
    pub row_id: RowId,
}
```

Every repository method requires `&DbSite`, ensuring data isolation across sites.

### Repository Pattern

Zero-sized repository structs provide stateless database operations:

```rust
pub struct PostRepository;

impl Repository for PostRepository {
    type Entity = AnyPostWithEditContext;
    // ... trait implementation
}

impl PostRepository {
    // Custom methods
    pub fn select_by_post_id(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        post_id: PostId,
    ) -> Result<DbAnyPostWithEditContext, SqliteDbError> {
        // ...
    }
}
```

### Transaction Safety

Type-level transaction safety prevents nesting:

- `Connection` implements `TransactionManager` + `QueryExecutor`
- `Transaction` implements only `QueryExecutor`

```rust
// ✅ Safe: Connection can create transactions
let tx = conn.transaction()?;

// ❌ Compiler error: Transaction cannot create nested transactions
let nested = tx.transaction()?; // Won't compile
```

## File Organization

```
wp_mobile_cache/
├── docs/                      # Documentation (you are here)
├── migrations/
│   ├── 0001-create-sites-table.sql
│   ├── 0002-create-posts-table.sql
│   └── 0003-create-term-relationships.sql
├── src/
│   ├── repository/
│   │   ├── mod.rs            # Core traits
│   │   ├── posts.rs          # PostRepository
│   │   └── term_relationships.rs
│   ├── mappings/
│   │   ├── posts.rs          # SQL row mapping
│   │   └── term_relationships.rs
│   ├── term_relationships.rs # DbTermRelationship type
│   └── lib.rs                # DbSite, RowId, migrations
└── REPOSITORY_PATTERN_DESIGN.md  # Legacy (archived)
```

## Contributing

When adding new entities:

1. Read the [Migration Guide](migration-guide.md)
2. Follow patterns from existing repositories
3. Document design decisions if making architectural choices
4. Add comprehensive tests

## Additional Resources

- [WordPress REST API Reference](https://developer.wordpress.org/rest-api/reference/)
- [SQLite Documentation](https://www.sqlite.org/docs.html)
- [Repository Pattern](https://martinfowler.com/eaaCatalog/repository.html)
