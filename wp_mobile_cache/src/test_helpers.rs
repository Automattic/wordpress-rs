//! Test helpers and fixtures for wp_mobile_cache tests.
//!
//! This module provides rstest fixtures and utilities to reduce boilerplate
//! in tests and make test intent clearer.
//!
//! # Usage
//!
//! ```rust
//! use rstest::*;
//! use wp_mobile_cache::test_helpers::*;
//!
//! #[rstest]
//! fn test_something(test_db: Connection, test_site: DbSite, post_repo: PostRepository) {
//!     let post = PostBuilder::new().with_author(UserId(10)).build();
//!     post_repo.upsert(&mut test_db, &test_site, &post, &test_site).unwrap();
//! }
//! ```

use crate::{
    DbSite, MigrationManager, RowId,
    repository::{posts::PostRepository, term_relationships::TermRelationshipRepository},
};
use rstest::*;
use rusqlite::Connection;

/// Fixture: Creates an in-memory test database with all migrations applied.
///
/// Also inserts a test site with id = 1 to satisfy foreign key constraints.
#[fixture]
pub fn test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    let mut migration_manager = MigrationManager::new(&conn).unwrap();

    migration_manager
        .perform_migrations()
        .expect("All migrations should succeed");

    // Insert default test site (id = 1)
    conn.execute("INSERT INTO sites (id) VALUES (1)", [])
        .expect("Failed to insert test site");

    conn
}

/// Test context bundling common test dependencies.
///
/// Reduces boilerplate when you need connection, site, and repositories.
///
/// # Example
///
/// ```rust
/// #[rstest]
/// fn test_something(test_ctx: TestContext) {
///     let post = PostBuilder::new().build();
///     test_ctx.post_repo.upsert(&mut test_ctx.conn, &test_ctx.site, &post).unwrap();
/// }
/// ```
pub struct TestContext {
    pub conn: Connection,
    pub site: DbSite,
    pub post_repo: PostRepository,
    pub term_repo: TermRelationshipRepository,
}

#[fixture]
pub fn test_ctx(test_db: Connection) -> TestContext {
    TestContext {
        conn: test_db,
        site: DbSite { row_id: RowId(1) },
        post_repo: PostRepository,
        term_repo: TermRelationshipRepository,
    }
}

/// Helper to create an additional test site with a specific ID.
///
/// Useful when you need more than 2 sites or specific site IDs.
pub fn create_test_site(conn: &Connection, id: i64) -> DbSite {
    conn.execute("INSERT INTO sites (id) VALUES (?)", [id])
        .expect("Failed to create test site");
    DbSite {
        row_id: RowId(id as u64),
    }
}
