use crate::{
    MigrationManager,
    context::EditContext,
    db_types::{db_site::DbSite, self_hosted_site::SelfHostedSite},
    repository::{
        posts::PostRepository, sites::SiteRepository,
        term_relationships::TermRelationshipRepository,
    },
};
use chrono::{DateTime, Utc};
use integration_test_credentials::TestCredentials;
use rstest::*;
use rusqlite::Connection;
use std::sync::atomic::{AtomicU32, Ordering};

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
    let (conn, site) = test_db();
    TestContext {
        conn,
        site,
        post_repo: PostRepository::new(),
        term_repo: TermRelationshipRepository,
    }
}

fn test_db() -> (Connection, DbSite) {
    let conn = Connection::open_in_memory().unwrap();
    let mut migration_manager = MigrationManager::new(&conn).unwrap();

    migration_manager
        .perform_migrations()
        .expect("All migrations should succeed");

    // Insert default test site using real test credentials
    let test_creds = TestCredentials::instance();
    let self_hosted_site = SelfHostedSite {
        url: test_creds.site_url.to_string(),
        api_root: format!("{}/wp-json", test_creds.site_url),
    };

    let db_site = create_test_site(&conn, &self_hosted_site);

    (conn, db_site)
}

/// Helper to create a test site.
///
/// Uses `SiteRepository` to insert the site into the database.
pub fn create_test_site(conn: &Connection, site: &SelfHostedSite) -> DbSite {
    let site_repo = SiteRepository;
    let (db_site, _) = site_repo
        .upsert_self_hosted_site(conn, site)
        .expect("Failed to upsert test site");

    db_site
}

static RANDOM_TEST_SITE_COUNTER: AtomicU32 = AtomicU32::new(1);

/// Helper to create a test site with auto-generated URL.
///
/// Uses an internal counter to generate unique URLs for each call.
/// Useful for tests that need multiple sites but don't care about specific URLs.
///
/// # Example
///
/// ```rust
/// let site1 = create_random_test_site(&conn);
/// let site2 = create_random_test_site(&conn);
/// // site1 and site2 will have different URLs
/// ```
pub fn create_random_test_site(conn: &Connection) -> DbSite {
    let counter = RANDOM_TEST_SITE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let site = SelfHostedSite {
        url: format!("https://test-site-{}.local", counter),
        api_root: format!("https://test-site-{}.local/wp-json", counter),
    };
    create_test_site(conn, &site)
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

/// Extract column names from a table using SQLite's PRAGMA table_info.
///
/// Returns a vector of column names in the order they appear in the table schema.
/// This is useful for verifying that column enums match the actual database schema.
///
/// # Example
///
/// ```rust
/// use PostEditContextColumn::*;
///
/// let columns = get_table_column_names(&conn, "posts_edit_context");
/// assert_eq!(columns[Rowid.as_index()], "rowid");
/// assert_eq!(columns[SiteId.as_index()], "db_site_id");
/// ```
pub fn get_table_column_names(conn: &Connection, table_name: &str) -> Vec<String> {
    let query = format!("PRAGMA table_info({})", table_name);
    conn.prepare(&query)
        .expect("Failed to prepare PRAGMA query")
        .query_map([], |row| row.get::<_, String>(1)) // column name is at index 1
        .expect("Failed to execute PRAGMA query")
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to collect column names")
}
