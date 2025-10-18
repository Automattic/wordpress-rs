# Documentation Reorganization Task

## Context

The `REPOSITORY_PATTERN_DESIGN.md` file has grown quite large as we've added more design decisions (currently 9 decisions documented). It's becoming unwieldy and harder to navigate.

## Goal

Reorganize the documentation into a more maintainable structure by:
1. Creating a `wp_mobile_cache/docs/` folder
2. Splitting the monolithic design document into focused, topic-specific documents
3. Creating a clear navigation/index for the documentation

## Current Structure

`REPOSITORY_PATTERN_DESIGN.md` contains:
- Overview
- Goals and Non-Goals
- Architecture (Core Components)
- 9 Design Decisions
- Implementation Plan
- Usage Examples
- Future Enhancements
- File Organization
- Conclusion

## Proposed Structure

```
wp_mobile_cache/
├── docs/
│   ├── README.md                           # Overview, navigation, getting started
│   ├── architecture/
│   │   ├── core-traits.md                  # QueryExecutor, TransactionManager, DbEntity, Repository
│   │   ├── database-schema.md              # Complete schema with all tables and indexes
│   │   └── type-system.md                  # RowId, DbSite, DbTermRelationship, etc.
│   ├── design-decisions/
│   │   ├── 01-executor-passing.md          # Pass executor explicitly vs storing
│   │   ├── 02-associated-types.md          # Associated type vs generic parameter
│   │   ├── 03-zero-sized-repos.md          # Zero-sized repository structs
│   │   ├── 04-minimal-abstraction.md       # QueryExecutor abstraction strategy
│   │   ├── 05-entity-vs-wrapper.md         # Domain entities vs database wrappers
│   │   ├── 06-upsert-pattern.md            # UPSERT for insert/update operations
│   │   ├── 07-multi-site-dbsite.md         # DbSite instead of simple ID
│   │   ├── 08-split-traits.md              # QueryExecutor vs TransactionManager
│   │   └── 09-term-normalization.md        # Term relationships normalization
│   ├── repositories/
│   │   ├── post-repository.md              # PostRepository API and usage
│   │   └── term-relationship-repository.md # TermRelationshipRepository API
│   ├── usage-examples.md                   # Code examples for common operations
│   └── migration-guide.md                  # How to add new entities/tables
├── src/
│   └── ...
└── REPOSITORY_PATTERN_DESIGN.md            # Keep as legacy or remove after migration
```

## Requirements

1. **Maintain all content**: Don't lose any information during reorganization
2. **Add cross-references**: Link between related documents
3. **Update file paths**: The File Organization section should reflect the new docs structure
4. **Keep examples intact**: All code examples should remain accurate
5. **Create index**: `docs/README.md` should provide clear navigation to all topics
6. **Preserve context**: Each document should be self-contained enough to understand independently
7. **Add timestamps**: Consider adding "Last Updated" dates to track freshness

## Success Criteria

- [ ] All design decisions are documented in individual, focused files
- [ ] Navigation is clear and intuitive
- [ ] Code examples are preserved and accurate
- [ ] Cross-references help readers navigate related topics
- [ ] Each document can be read independently without too much jumping around
- [ ] File organization reflects actual codebase structure

## Optional Enhancements

- Add diagrams (using mermaid or similar) for architecture visualization
- Create a decision log template for future design decisions
- Add a glossary for terms like "rowid", "DbSite", "executor", etc.
- Consider adding sequence diagrams for operations like "upsert with terms"

## Notes

- The current design document is comprehensive but monolithic
- Some decisions build on earlier ones (e.g., Decision 7 references Decision 1)
- Usage examples should probably stay together in one file for easy reference
- Consider keeping a high-level README that links to deeper topics

## Starting Point

Begin by reading `wp_mobile_cache/REPOSITORY_PATTERN_DESIGN.md` to understand the full scope, then create the new structure progressively, migrating sections as you go.
