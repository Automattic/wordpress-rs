use crate::{
    DbSite, MigrationManager, RowId,
    context::EditContext,
    repository::{posts::PostRepository, term_relationships::TermRelationshipRepository},
};
use chrono::{DateTime, Utc};
use rstest::*;
use rusqlite::Connection;

pub mod posts;

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
    pub post_repo: PostRepository<EditContext>,
    pub term_repo: TermRelationshipRepository,
}

#[fixture]
pub fn test_ctx() -> TestContext {
    TestContext {
        conn: test_db(),
        site: DbSite { row_id: RowId(1) },
        post_repo: PostRepository::new(),
        term_repo: TermRelationshipRepository,
    }
}

fn test_db() -> Connection {
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

/// Validates that a timestamp is a recent, valid ISO 8601 UTC timestamp.
///
/// Checks that the timestamp:
/// - Is in valid ISO 8601 format
/// - Is in UTC (ends with 'Z')
/// - Is within the last 5 seconds of the current time
///
/// # Panics
///
/// Panics if the timestamp is invalid or not recent.
pub fn assert_recent_timestamp(timestamp: &str) {
    // Parse the timestamp
    let parsed = DateTime::parse_from_rfc3339(timestamp)
        .unwrap_or_else(|e| panic!("Failed to parse timestamp '{}': {}", timestamp, e));

    // Verify it's UTC (ends with Z)
    assert!(
        timestamp.ends_with('Z'),
        "Timestamp should be UTC (end with Z): {}",
        timestamp
    );

    // Check that it's recent (within last 5 seconds)
    let now = Utc::now();
    let timestamp_utc = parsed.with_timezone(&Utc);
    let diff = now.signed_duration_since(timestamp_utc);

    assert!(
        diff.num_seconds() >= 0 && diff.num_seconds() <= 5,
        "Timestamp should be within last 5 seconds. Now: {}, Timestamp: {}, Diff: {} seconds",
        now,
        timestamp_utc,
        diff.num_seconds()
    );
}
