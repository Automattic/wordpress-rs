# Adding Database Support for New WordPress Entities

This guide explains how to add caching support for WordPress REST API entity types (posts, comments, users, etc.).

## Overview

WordPress REST API returns different fields based on the `context` parameter (`edit`, `view`, `embed`). Each context requires its own database table since field sets differ significantly.

**Architecture:**
- `context.rs` - Generic `IsContext` trait and context marker types (EditContext, ViewContext, EmbedContext)
- `db_types/{entity}/` - Column enums and wrapper structs (types only, no logic)
- `repository/{entity}.rs` - Entity-specific trait definitions and all database operations

Use the existing `posts` implementation in `src/repository/posts.rs` as the primary reference.

## Discovering Context Fields

The `wp_api` crate uses procedural macros to generate context-specific types. To see what fields are actually available:

```bash
# Generate expanded code for an entity
cargo expand -p wp_api posts > /tmp/generated_posts.rs
# or
cargo expand -p wp_api comments > /tmp/generated_comments.rs
```

Open the generated file and search for the context-specific types (e.g., `AnyPostWithEditContext`, `AnyPostWithViewContext`, `AnyPostWithEmbedContext`). Compare fields across contexts to understand what differs.

**Important**: Don't assume patterns - field availability varies by endpoint. Always verify against the expanded types.

## Implementation Steps

When adding a new entity type (e.g., implementing cache support for comments):

1. **Create type definitions** in `src/db_types/comments/`:
   - `mod.rs` - Re-exports
   - `edit.rs`, `view.rs`, `embed.rs` - One file per context
   - Each file contains: column enum (matching SQL table order) + database wrapper struct

2. **Define entity-specific trait and implement database logic** in `src/repository/comments.rs`:
   - Define `CommentContext` trait extending `IsContext` with associated types and row mapping method
   - The method signature depends on your entity's needs (e.g., posts use lazy closures for term loading)
   - Implement trait for each context (EditContext, ViewContext, EmbedContext)
   - Create generic `CommentRepository<C: CommentContext>`
   - Implement upsert methods for contexts that need write operations

3. **Create migrations** for each context table:
   - `migrations/NNNN-create-comments-edit-context-table.sql`
   - `migrations/NNNN-create-comments-view-context-table.sql`
   - `migrations/NNNN-create-comments-embed-context-table.sql`
   - Add to `MIGRATION_QUERIES` array in `lib.rs`
   - Update migration count assertions in platform test files:
     - `native/swift/Tests/wordpress-api-cache/WordPressApiCacheTests.swift`
     - `native/kotlin/api/kotlin/src/integrationTest/kotlin/WordPressApiCacheTest.kt`

4. **Add module exports** in `src/db_types/mod.rs`:
   ```rust
   pub mod comments;
   ```

5. **Write tests** in the repository file covering round-trip persistence and enum variants

## Key Design Decisions

**Column enum ordering**: Column indexes in the enum must match the exact order in SQL `CREATE TABLE` statements and all `SELECT *` queries. Add PRAGMA-based integration tests to verify this (see `test_post_edit_context_column_enum_matches_schema` in `repository/posts.rs` for reference).

**Lazy data loading**: If your entity requires optional related data (e.g., posts use term relationships for categories/tags), use `FnOnce()` closures in the trait method. This allows contexts that don't need the data to avoid unnecessary database queries by simply not calling the closure. See `PostContext::from_row_with_terms` for an example.

**Repository pattern**: All database operations belong in `repository/` - this includes both SQL execution AND row-to-type mapping logic. The `db_types/` module contains only type definitions.

**RETURNING optimization**: Use SQLite's `RETURNING rowid` clause in upsert statements to eliminate separate SELECT queries. Our bundled SQLite version (3.50.2) fully supports this.

## Reference Implementation

See `src/repository/posts.rs` for a complete working example including:
- Context trait implementations with lazy closure pattern
- Generic repository with context-specific methods
- PRAGMA tests for column schema verification
- Comprehensive test coverage
