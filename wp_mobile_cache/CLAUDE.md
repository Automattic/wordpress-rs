# wp_mobile_cache - Claude Instructions

This crate provides SQLite-based caching for WordPress REST API data with support for multiple API contexts (edit, view, embed).

## Architecture Overview

- **Mappings** (`src/mappings/`): Convert between wp_api types and SQLite rows
- **Repositories** (`src/repository/`): CRUD operations for cached entities
- **Migrations** (`migrations/`): SQLite schema definitions

Each WordPress entity type (posts, comments, etc.) has separate tables for each API context (edit, view, embed) because they contain different fields.

## Key Concepts

**Context-Specific Tables**: WordPress API returns different fields based on context parameter
- Tables are named: `{entity}_{context}_context` (e.g., `posts_edit_context`, `posts_view_context`)
- Each context needs its own database mapping implementation
- Field availability varies by endpoint - always verify by expanding the actual wp_api types

**Mappings Organization**:
```
src/mappings/posts/
├── mod.rs      # Re-exports
├── edit.rs     # EditContext mapping
├── view.rs     # ViewContext mapping
└── embed.rs    # EmbedContext mapping
```

## Common Tasks

### Implementing Mappings for a New Entity Type

Example: "Implement mappings for SparseComment in wp_api/src/comments.rs"

**Quick steps:**
1. Generate concrete types: `cargo expand -p wp_api comments > /tmp/generated_comments.rs`
2. Find types: `grep "pub struct.*Comment.*Context" /tmp/generated_comments.rs`
3. Analyze field differences between Edit/View/Embed contexts
4. Create `src/mappings/comments/{mod,edit,view,embed}.rs` following `src/mappings/posts/` pattern
5. Create migration files for each context table
6. Use helper functions from `src/mappings/helpers.rs`

**Detailed guide**: See `docs/ADDING_NEW_ENTITIES.md` for complete step-by-step instructions.

### Understanding Generated Types

The `wp_api` crate uses procedural macros to generate context-specific types:
- Source: `Sparse*` types with `#[WpContextual]` macro
- Generated: `*WithEditContext`, `*WithViewContext`, `*WithEmbedContext`

To see what fields are actually generated, use `cargo expand -p wp_api {module_name}`.

### Available Mapping Helpers

All in `src/mappings/helpers.rs`:
- `get_id()`, `get_optional_id()` - Extract ID types
- `parse_datetime()`, `parse_enum()` - Parse specific types
- `integer_to_bool()`, `bool_to_integer()` - Boolean conversions
- `serialize_value_to_json()`, `deserialize_json_value()` - JSON handling

## Important Files

- `docs/ADDING_NEW_ENTITIES.md` - Complete guide for adding database support for new entity types
- `CONTEXT_FIELD_ANALYSIS.md` - Example field analysis for posts
- `CONTEXT_SUPPORT_DESIGN.md` - Architecture design document
- `src/mappings/posts/edit.rs` - Reference implementation for mappings

## Database Schema Conventions

- All tables use `STRICT` mode
- Entity IDs are WordPress IDs (from API), not SQLite rowids
- Unique constraint on `(db_site_id, id)` ensures one entity per site
- Nested types are flattened (e.g., `title.raw` → `title_raw` column)
- `last_fetched_at` tracks cache freshness
- Foreign key to `sites` table with `ON DELETE CASCADE`

## Testing

Tests in `src/repository/{entity}.rs` should cover:
- Round-trip persistence (insert → select → compare)
- All enum variants
- Optional fields (None and Some cases)
- Term relationships (if applicable)
- Empty collections vs None

Use `rstest` for parameterized tests and test fixtures from `src/test_fixtures/`.
