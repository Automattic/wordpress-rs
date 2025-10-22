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
//!     post_repo.insert(&test_db, &post, &test_site).unwrap();
//! }
//! ```

use crate::{DbSite, MigrationManager, RowId, repository::posts::PostRepository};
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

/// Fixture: Default test site with row_id = 1.
///
/// Note: Depends on `test_db` fixture which creates site with id = 1.
#[fixture]
pub fn test_site() -> DbSite {
    DbSite { row_id: RowId(1) }
}

/// Fixture: Second test site with row_id = 2 for multi-site testing.
///
/// Creates an additional site in the database to test site isolation.
#[fixture]
pub fn second_site(test_db: Connection) -> (Connection, DbSite) {
    test_db
        .execute("INSERT INTO sites (id) VALUES (2)", [])
        .expect("Failed to insert second test site");
    (test_db, DbSite { row_id: RowId(2) })
}

/// Fixture: PostRepository instance.
///
/// Zero-cost abstraction since PostRepository is zero-sized.
#[fixture]
pub fn post_repo() -> PostRepository {
    PostRepository
}

/// Test context bundling common test dependencies.
///
/// Reduces boilerplate when you need all three: connection, site, and repository.
///
/// # Example
///
/// ```rust
/// #[rstest]
/// fn test_something(test_ctx: TestContext) {
///     let post = PostBuilder::new().build();
///     test_ctx.repo.insert(&test_ctx.conn, &post, &test_ctx.site).unwrap();
/// }
/// ```
pub struct TestContext {
    pub conn: Connection,
    pub site: DbSite,
    pub repo: PostRepository,
}

#[fixture]
pub fn test_ctx(test_db: Connection, test_site: DbSite, post_repo: PostRepository) -> TestContext {
    TestContext {
        conn: test_db,
        site: test_site,
        repo: post_repo,
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
