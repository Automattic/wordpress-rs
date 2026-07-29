use rusqlite::functions::FunctionFlags;
use rusqlite::hooks::Action;
use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput};
use rusqlite::{Connection, Result as SqliteResult, params};
use std::sync::{Arc, Mutex};

use crate::repository::entity_state::EntityStateRepository;
use crate::repository::list_metadata::ListMetadataRepository;
use crate::repository::sites::SiteRepository;
use wp_api::parsed_url::ParsedUrl;
use wp_api::wp_com::WpComSiteId;

pub mod context;
pub mod db_types;
pub mod entity;
pub mod list_metadata;
pub mod repository;
pub mod term_relationships;

#[cfg(any(test, feature = "test-helpers"))]
pub mod test_fixtures;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum SqliteDbError {
    SqliteError(String),
    ConstraintViolation(String),
    TableNameMismatch { expected: DbTable, actual: DbTable },
    PerPageMismatch { expected: i64, actual: i64 },
}

impl std::fmt::Display for SqliteDbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SqliteDbError::SqliteError(message) => write!(f, "SqliteDbError: message={}", message),
            SqliteDbError::ConstraintViolation(message) => {
                write!(f, "Constraint violation: {}", message)
            }
            SqliteDbError::TableNameMismatch { expected, actual } => {
                write!(
                    f,
                    "Table mismatch: expected '{}', but got '{}'",
                    expected.table_name(),
                    actual.table_name()
                )
            }
            SqliteDbError::PerPageMismatch { expected, actual } => {
                write!(
                    f,
                    "per_page mismatch: expected {}, but list has {}",
                    expected, actual
                )
            }
        }
    }
}

impl From<rusqlite::Error> for SqliteDbError {
    fn from(err: rusqlite::Error) -> Self {
        if let rusqlite::Error::SqliteFailure(sqlite_err, _) = &err
            && sqlite_err.code == rusqlite::ErrorCode::ConstraintViolation
        {
            return SqliteDbError::ConstraintViolation(err.to_string());
        }
        SqliteDbError::SqliteError(err.to_string())
    }
}

/// Database table identifier.
///
/// Represents all tables tracked in the cache database. This enum provides
/// a single source of truth for table names, ensuring consistency across
/// the codebase and preventing typos.
///
/// Note: uniffi::Enum makes this type available to platform bindings but
/// without exposing any methods (which is intentional - we don't want
/// Kotlin/Swift to access table names directly).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
#[non_exhaustive]
pub enum DbTable {
    /// Posts with edit context (full data for editing)
    PostsEditContext,
    /// Posts with view context (public viewing data)
    PostsViewContext,
    /// Posts with embed context (minimal data for embeds)
    PostsEmbedContext,
    /// Post types with edit context (post type configuration data)
    PostTypesEditContext,
    /// Media with edit context (full media data for editing)
    MediaEditContext,
    /// Self-hosted WordPress sites
    SelfHostedSites,
    /// Database sites mapping table
    DbSites,
    /// Term relationships (post-category, post-tag associations)
    TermRelationships,
    /// List metadata headers (pagination, version)
    ListMetadata,
    /// List metadata items (entity IDs with ordering)
    ListMetadataItems,
    /// List metadata sync state (idle, fetching, error)
    ListMetadataState,
    /// Entity state tracking (missing, fetching, cached, stale, failed)
    EntityState,
    /// WordPress.com sites
    WordPressComSites,
}

impl DbTable {
    /// Get the database table name as a string.
    ///
    /// This is the only place where table names are defined as strings,
    /// ensuring single source of truth for all SQL queries.
    pub fn table_name(&self) -> &'static str {
        match self {
            DbTable::PostsEditContext => "posts_edit_context",
            DbTable::PostsViewContext => "posts_view_context",
            DbTable::PostsEmbedContext => "posts_embed_context",
            DbTable::PostTypesEditContext => "post_types_edit_context",
            DbTable::MediaEditContext => "media_edit_context",
            DbTable::SelfHostedSites => "self_hosted_sites",
            DbTable::DbSites => "db_sites",
            DbTable::TermRelationships => "term_relationships",
            DbTable::ListMetadata => "list_metadata",
            DbTable::ListMetadataItems => "list_metadata_items",
            DbTable::ListMetadataState => "list_metadata_state",
            DbTable::EntityState => "entity_state",
            DbTable::WordPressComSites => "wordpress_com_sites",
        }
    }
}

impl std::fmt::Display for DbTable {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.table_name())
    }
}

/// Error type for DbTable conversion failures
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DbTableError {
    #[error("Unknown table: {0}")]
    UnknownTable(String),
}

impl TryFrom<&str> for DbTable {
    type Error = DbTableError;

    fn try_from(table_name: &str) -> Result<Self, Self::Error> {
        match table_name {
            "posts_edit_context" => Ok(DbTable::PostsEditContext),
            "posts_view_context" => Ok(DbTable::PostsViewContext),
            "posts_embed_context" => Ok(DbTable::PostsEmbedContext),
            "post_types_edit_context" => Ok(DbTable::PostTypesEditContext),
            "media_edit_context" => Ok(DbTable::MediaEditContext),
            "self_hosted_sites" => Ok(DbTable::SelfHostedSites),
            "db_sites" => Ok(DbTable::DbSites),
            "term_relationships" => Ok(DbTable::TermRelationships),
            "list_metadata" => Ok(DbTable::ListMetadata),
            "list_metadata_items" => Ok(DbTable::ListMetadataItems),
            "list_metadata_state" => Ok(DbTable::ListMetadataState),
            "entity_state" => Ok(DbTable::EntityState),
            "wordpress_com_sites" => Ok(DbTable::WordPressComSites),
            _ => Err(DbTableError::UnknownTable(table_name.to_string())),
        }
    }
}

uniffi::custom_newtype!(RowId, i64);

/// Represents a database row ID (autoincrement field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RowId(pub i64);

impl ToSql for RowId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0))
    }
}

impl FromSql for RowId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> FromSqlResult<Self> {
        i64::column_result(value).map(|i| {
            debug_assert!(i >= 0, "RowId should be non-negative, got: {}", i);
            RowId(i)
        })
    }
}

impl From<i64> for RowId {
    fn from(value: i64) -> Self {
        debug_assert!(value >= 0, "RowId should be non-negative, got: {}", value);
        RowId(value)
    }
}

impl RowId {
    /// Convert a slice of RowIds to a comma-separated string for use in SQL IN clauses.
    ///
    /// This helper is used when building dynamic SQL queries with arrays of IDs.
    /// Since RowIds are internal database IDs (not user input), this is safe from SQL injection.
    ///
    /// # Example
    /// ```ignore
    /// let row_ids = vec![RowId(1), RowId(2), RowId(3)];
    /// let ids_str = RowId::to_sql_list(&row_ids); // "1, 2, 3"
    /// let sql = format!("SELECT * FROM table WHERE id IN ({})", ids_str);
    /// ```
    pub fn to_sql_list(row_ids: &[RowId]) -> String {
        row_ids
            .iter()
            .map(|id| id.0.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl From<RowId> for i64 {
    fn from(row_id: RowId) -> Self {
        row_id.0
    }
}

/// Get the SQLite version string from the database.
///
/// This function queries the database for its SQLite version using `SELECT sqlite_version()`.
/// Clients can use this to check compatibility or log version information.
///
/// # Example
/// ```ignore
/// use wp_mobile_cache::get_sqlite_version;
///
/// let version = get_sqlite_version(&connection)?;
/// println!("SQLite version: {}", version);
/// ```
pub fn get_sqlite_version(
    executor: &impl repository::QueryExecutor,
) -> Result<String, SqliteDbError> {
    let mut stmt = executor.prepare("SELECT sqlite_version()")?;
    stmt.query_row([], |row| row.get(0))
        .map_err(SqliteDbError::from)
}

/// Check if a SQLite version supports the RETURNING clause.
///
/// The RETURNING clause was added in SQLite 3.35.0. This function parses the version
/// string and returns true if the version is >= 3.35.0.
///
/// # Arguments
/// * `version` - SQLite version string in the format "X.Y.Z" (e.g., "3.45.0")
///
/// # Returns
/// * `true` if the version supports RETURNING (>= 3.35.0)
/// * `false` if the version is too old or if the version string cannot be parsed
///
/// # Example
/// ```ignore
/// use wp_mobile_cache::{get_sqlite_version, does_sqlite_support_returning};
///
/// let version = get_sqlite_version(&connection)?;
/// if does_sqlite_support_returning(&version) {
///     // Safe to use RETURNING clause
/// }
/// ```
pub fn does_sqlite_support_returning(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 2 {
        return false;
    }

    let Ok(major) = parts[0].parse::<i32>() else {
        return false;
    };
    let Ok(minor) = parts[1].parse::<i32>() else {
        return false;
    };

    // RETURNING was added in SQLite 3.35.0
    if major > 3 {
        return true;
    }
    if major == 3 && minor >= 35 {
        return true;
    }

    false
}

#[derive(uniffi::Object)]
pub struct WpApiCache {
    inner: DBManager,
}

#[uniffi::export]
impl WpApiCache {
    #[uniffi::constructor]
    pub fn new(path: Option<String>) -> Result<Self, SqliteDbError> {
        Ok(Self {
            inner: DBManager::new(&path)?,
        })
    }

    pub fn perform_migrations(&self) -> Result<i64, SqliteDbError> {
        self.execute(|connection| {
            let mut mgr = MigrationManager::new(connection)?;
            let version = mgr.perform_migrations().map_err(SqliteDbError::from)?;

            // Reset stale fetching states after migrations complete.
            // Errors are logged but not propagated: this is a best-effort cleanup,
            // and failure doesn't affect core functionality (worst case: UI shows
            // stale loading state).
            if let Err(e) = ListMetadataRepository::reset_stale_fetching_states(connection) {
                log::warn!("Failed to reset stale fetching states: {}", e);
            }

            // Clear abandoned fetch operations after migrations complete.
            // Deletes Fetching states to prevent stuck loading indicators while
            // preserving Fresh states to avoid unnecessary refetching.
            if let Err(e) = EntityStateRepository::clear_abandoned_fetches(connection) {
                log::warn!("Failed to clear abandoned fetches: {}", e);
            }

            Ok(version)
        })
    }

    pub fn start_listening_for_updates(&self, delegate: std::sync::Arc<dyn DatabaseDelegate>) {
        self.execute(|connection| {
            connection.update_hook(Some(
                move |action: Action, db_name: &str, table_name: &str, row_id: i64| {
                    match DbTable::try_from(table_name) {
                        Ok(table) => {
                            let hook_data = UpdateHook {
                                action: action.into(),
                                db_name: db_name.to_string(),
                                table,
                                row_id,
                            };
                            delegate.did_update(hook_data);
                        }
                        Err(_) => {
                            // Ignore SQLite system tables (sqlite_sequence, sqlite_master, etc.)
                            // and migration tracking table (_migrations)
                            if !table_name.starts_with("sqlite_") && table_name != "_migrations" {
                                log::warn!("Unknown table in update hook: {}", table_name);
                            }
                        }
                    }
                },
            ));
        });
    }

    pub fn stop_listening_for_updates(&self) {
        self.execute(|connection| {
            connection.update_hook(None::<fn(Action, &str, &str, i64)>);
        });
    }

    /// Remove a self-hosted site and all its cached data from the database.
    ///
    /// Returns `true` if the site was found and removed, `false` if no site
    /// with the given URL exists.
    pub fn remove_self_hosted_site(&self, url: Arc<ParsedUrl>) -> Result<bool, SqliteDbError> {
        self.execute(|connection| {
            let Some(full_entity) =
                SiteRepository.select_self_hosted_site_by_url(connection, &url.url())?
            else {
                return Ok(false);
            };

            SiteRepository.delete_site(connection, &full_entity.data.0)
        })
    }

    /// Remove a WordPress.com site and all its cached data from the database.
    ///
    /// Returns `true` if the site was found and removed, `false` if no site
    /// with the given site ID exists.
    pub fn remove_wordpress_com_site(&self, site_id: WpComSiteId) -> Result<bool, SqliteDbError> {
        self.execute(|connection| {
            SiteRepository.delete_wordpress_com_site_by_site_id(connection, site_id)
        })
    }
}

impl WpApiCache {
    /// Execute a database operation with scoped access to the connection.
    ///
    /// This is the **only** way to access the database. The provided closure
    /// receives a mutable reference to the connection that is only valid within
    /// the closure scope.
    ///
    /// This design prevents deadlocks by ensuring that:
    /// 1. The connection cannot be held across multiple `execute()` calls
    /// 2. The closure scope naturally encourages grouping related operations
    /// 3. The lock is automatically released when the closure returns
    ///
    /// # Example
    /// ```ignore
    /// // Read operation
    /// let post = cache.execute(|conn| {
    ///     repo.select_by_id(conn, post_id)
    /// })?;
    ///
    /// // Write operation
    /// cache.execute(|conn| {
    ///     repo.upsert(conn, &site, &post)
    /// })?;
    /// ```
    ///
    /// # Preventing Deadlocks
    ///
    /// **Safe** - Lock is released between calls:
    /// ```ignore
    /// let data = cache.execute(|conn| read(conn))?;
    /// cache.execute(|conn| write(conn, data))?;  // ✅ OK
    /// ```
    ///
    /// **Unsafe** - Nested calls (requires explicit Arc clone, unlikely to write accidentally):
    /// ```ignore
    /// let cache2 = cache.clone();
    /// cache.execute(|conn| {
    ///     cache2.execute(|conn2| { ... })  // ⚠️ Deadlock!
    /// })?;
    /// ```
    pub fn execute<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Connection) -> R,
    {
        let mut conn = self.inner.connection.lock().unwrap();
        f(&mut conn)
    }
}

impl TryFrom<Connection> for WpApiCache {
    type Error = SqliteDbError;

    /// Create a WpApiCache from an existing connection.
    ///
    /// This is typically used in tests to create a cache from an already-migrated
    /// in-memory database connection.
    fn try_from(connection: Connection) -> Result<Self, Self::Error> {
        configure_connection(&connection)?;
        Ok(Self {
            inner: DBManager {
                connection: Mutex::new(connection),
            },
        })
    }
}

static MIGRATION_QUERIES: [&str; 14] = [
    include_str!("../migrations/0001-create-sites-table.sql"),
    include_str!("../migrations/0002-create-posts-table.sql"),
    include_str!("../migrations/0003-create-term-relationships.sql"),
    include_str!("../migrations/0004-create-posts-view-context-table.sql"),
    include_str!("../migrations/0005-create-posts-embed-context-table.sql"),
    include_str!("../migrations/0006-create-self-hosted-sites-table.sql"),
    include_str!("../migrations/0007-create-list-metadata-tables.sql"),
    include_str!("../migrations/0008-create-post-types-table.sql"),
    include_str!("../migrations/0009-create-entity-state-table.sql"),
    include_str!("../migrations/0010-create-wordpress-com-sites-table.sql"),
    include_str!("../migrations/0011-add-additional-fields-to-posts-tables.sql"),
    include_str!("../migrations/0012-invalidate-post-entity-states.sql"),
    include_str!("../migrations/0013-invalidate-post-entity-states-for-meta.sql"),
    include_str!("../migrations/0014-create-media-edit-context-table.sql"),
];

pub struct MigrationManager<'a> {
    connection: &'a Connection,
}

impl<'a> MigrationManager<'a> {
    pub fn new(connection: &'a Connection) -> Result<Self, SqliteDbError> {
        // Cover raw-connection paths that bypass WpApiCache construction
        // (test fixtures that hand a Connection straight to the repository
        // layer). The registration is idempotent, so connections that were
        // already prepared by DBManager / From<Connection> are unaffected.
        register_url_functions(connection)?;
        Ok(Self { connection })
    }

    pub fn has_migrations_table(&self) -> SqliteResult<bool> {
        let mut statement: rusqlite::Statement<'_> = self.connection.prepare(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_migrations'",
        )?;

        let result = statement.query_row([], |row| row.get::<_, i32>(0))?;

        Ok(result > 0)
    }

    pub fn perform_migrations(&mut self) -> SqliteResult<i64> {
        self.connection.execute("BEGIN EXCLUSIVE", ())?;

        match self.migrate() {
            Ok(count) => {
                self.connection.execute("COMMIT", ())?;
                Ok(count)
            }
            Err(e) => {
                let _ = self.connection.execute("ROLLBACK", ());
                Err(e)
            }
        }
    }

    fn migrate(&mut self) -> SqliteResult<i64> {
        if !self.has_migrations_table()? {
            self.create_migrations_table()?;
        }

        let next_migration_id = self.get_next_migration_index()?;
        if next_migration_id > MIGRATION_QUERIES.len() {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "next migration index ({next_migration_id}) exceeds available migrations ({})",
                MIGRATION_QUERIES.len()
            )));
        }
        for (index, migration) in MIGRATION_QUERIES[next_migration_id..].iter().enumerate() {
            for query in migration
                .split(";")
                .filter(|query| !query.trim().is_empty())
            {
                self.connection.execute(query, ())?;
            }

            // `.enumerate` will start the indexes from 0, so we need to add `next_migration_id`
            self.insert_migration((next_migration_id + index + 1) as i64)?;
        }

        Ok(MIGRATION_QUERIES[next_migration_id..].len() as i64)
    }

    pub fn create_migrations_table(&self) -> SqliteResult<()> {
        self.connection.execute(
            "CREATE TABLE _migrations (migration_id INTEGER PRIMARY KEY)",
            (),
        )?;
        Ok(())
    }

    pub fn insert_migration(&mut self, migration_id: i64) -> SqliteResult<()> {
        let mut insert_migration_query = self
            .connection
            .prepare("INSERT INTO _migrations (migration_id) VALUES (?)")?;
        insert_migration_query.execute(params![migration_id])?;
        Ok(())
    }

    /// Returns the index of the next migration to run in the `MIGRATION_QUERIES` array.
    /// Note that this is *not* the same as the migration ID.
    pub fn get_next_migration_index(&self) -> SqliteResult<usize> {
        let mut statement = self
            .connection
            .prepare("SELECT MAX(migration_id) FROM _migrations")?;
        let result = statement.query_row([], |row| row.get::<_, Option<usize>>(0))?;
        Ok(result.unwrap_or(0))
    }
}

#[uniffi::export(with_foreign)]
pub trait DatabaseDelegate: Send + Sync {
    fn did_update(&self, update_hook: UpdateHook);
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct User {
    id: i64,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct UpdateHook {
    pub action: HookAction,
    pub db_name: String,
    pub table: DbTable,
    pub row_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum HookAction {
    Insert,
    Update,
    Delete,
}

impl From<Action> for HookAction {
    fn from(action: Action) -> Self {
        match action {
            Action::SQLITE_INSERT => HookAction::Insert,
            Action::SQLITE_UPDATE => HookAction::Update,
            Action::SQLITE_DELETE => HookAction::Delete,
            _ => panic!("Invalid action: {:?}", action),
        }
    }
}

struct DBManager {
    /// Mutex-protected SQLite connection
    /// Access only through WpApiCache::execute() to prevent deadlocks
    connection: Mutex<Connection>,
}

impl DBManager {
    pub fn new(path: &Option<String>) -> Result<Self, SqliteDbError> {
        // Create the database connection
        let connection = if let Some(path) = path {
            Connection::open(path)?
        } else {
            Connection::open_in_memory()?
        };

        configure_connection(&connection)?;

        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
}

/// Register custom SQL functions on the given connection.
///
/// Currently registers `urls_eq(a, b)`: a deterministic, UTF-8 scalar that
/// reports semantic URL equivalence (per `ParsedUrl == ParsedUrl`), with a
/// raw-string fallback when either side fails to parse.
///
/// Why: the `self_hosted_sites.url` column may legitimately hold either
/// `http://example.com` or `http://example.com/` depending on whether it
/// was inserted before or after `ParsedUrl`-based normalization landed
/// (see PR #1239). Lookups must tolerate either shape without forcing a
/// data migration on existing user caches.
pub(crate) fn register_url_functions(conn: &Connection) -> SqliteResult<()> {
    conn.create_scalar_function(
        "urls_eq",
        2,
        FunctionFlags::SQLITE_DETERMINISTIC | FunctionFlags::SQLITE_UTF8,
        |ctx| {
            let a: String = ctx.get(0)?;
            let b: String = ctx.get(1)?;
            if a == b {
                return Ok(true);
            }
            Ok(match (ParsedUrl::parse(&a), ParsedUrl::parse(&b)) {
                (Ok(parsed_a), Ok(parsed_b)) => parsed_a == parsed_b,
                _ => false,
            })
        },
    )
}

fn configure_connection(connection: &Connection) -> Result<(), SqliteDbError> {
    register_url_functions(connection)?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    Ok(())
}

uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db_types::{
            db_site::DbSite, self_hosted_site::SelfHostedSite, wordpress_com_site::WordPressComSite,
        },
        repository::entity_state::{DbEntityState, EntityType},
    };

    #[derive(Debug, Default, PartialEq, Eq)]
    struct SiteRowCounts {
        db_sites: i64,
        list_metadata: i64,
        list_metadata_items: i64,
        list_metadata_state: i64,
        entity_state: i64,
    }

    impl SiteRowCounts {
        fn one_of_each() -> Self {
            Self {
                db_sites: 1,
                list_metadata: 1,
                list_metadata_items: 1,
                list_metadata_state: 1,
                entity_state: 1,
            }
        }
    }

    fn cache_from_connection_with_foreign_keys_disabled() -> WpApiCache {
        let connection = Connection::open_in_memory().expect("in-memory connection should open");
        connection
            .pragma_update(None, "foreign_keys", "OFF")
            .expect("foreign keys should be disabled before cache creation");
        MigrationManager::new(&connection)
            .expect("migration manager should be created")
            .perform_migrations()
            .expect("cache migrations should succeed");
        WpApiCache::try_from(connection).expect("cache should be created")
    }

    fn foreign_keys_are_enabled(cache: &WpApiCache) -> bool {
        cache.execute(|connection| {
            connection
                .pragma_query_value(None, "foreign_keys", |row| row.get::<_, bool>(0))
                .expect("foreign key setting should be readable")
        })
    }

    fn seed_site_records(cache: &WpApiCache, db_site: &DbSite, key: &str, entity_id: i64) -> i64 {
        cache.execute(|connection| {
            connection
                .execute(
                    "INSERT INTO list_metadata (db_site_id, key) VALUES (?1, ?2)",
                    params![db_site.row_id, key],
                )
                .expect("list metadata should be inserted");
            let list_metadata_id = connection.last_insert_rowid();
            connection
                .execute(
                    "INSERT INTO list_metadata_items (list_metadata_id, entity_id) VALUES (?1, ?2)",
                    params![list_metadata_id, entity_id],
                )
                .expect("list metadata item should be inserted");
            connection
                .execute(
                    "INSERT INTO list_metadata_state (list_metadata_id, state) VALUES (?1, 0)",
                    [list_metadata_id],
                )
                .expect("list metadata state should be inserted");
            EntityStateRepository::set_state(
                connection,
                entity_id,
                db_site,
                EntityType::PostsEditContext,
                &DbEntityState::Fresh,
            )
            .expect("entity state should be inserted");
            list_metadata_id
        })
    }

    fn row_counts(cache: &WpApiCache, db_site: &DbSite, list_metadata_id: i64) -> SiteRowCounts {
        cache.execute(|connection| {
            connection
                .query_row(
                    "SELECT
                        (SELECT COUNT(*) FROM db_sites WHERE rowid = ?1),
                        (SELECT COUNT(*) FROM list_metadata WHERE db_site_id = ?1),
                        (SELECT COUNT(*) FROM list_metadata_items
                         WHERE list_metadata_id = ?2),
                        (SELECT COUNT(*) FROM list_metadata_state
                         WHERE list_metadata_id = ?2),
                        (SELECT COUNT(*) FROM entity_state WHERE db_site_id = ?1)",
                    params![db_site.row_id, list_metadata_id],
                    |row| {
                        Ok(SiteRowCounts {
                            db_sites: row.get(0)?,
                            list_metadata: row.get(1)?,
                            list_metadata_items: row.get(2)?,
                            list_metadata_state: row.get(3)?,
                            entity_state: row.get(4)?,
                        })
                    },
                )
                .expect("site-scoped rows should be counted")
        })
    }

    fn assert_site_removal_cascades(
        cache: &WpApiCache,
        removed_site: &DbSite,
        preserved_site: &DbSite,
        remove_site: impl FnOnce() -> Result<bool, SqliteDbError>,
    ) {
        let removed_list_metadata_id = seed_site_records(cache, removed_site, "removed-list", 41);
        let preserved_list_metadata_id =
            seed_site_records(cache, preserved_site, "preserved-list", 42);

        assert!(remove_site().expect("site cleanup should succeed"));

        assert_eq!(
            row_counts(cache, removed_site, removed_list_metadata_id),
            SiteRowCounts::default()
        );
        assert_eq!(
            row_counts(cache, preserved_site, preserved_list_metadata_id),
            SiteRowCounts::one_of_each()
        );
    }

    #[test]
    fn direct_api_remove_self_hosted_site_cascades_with_foreign_keys_enabled() {
        let cache = cache_from_connection_with_foreign_keys_disabled();
        let removed_url =
            ParsedUrl::parse("https://removed.example.com").expect("removed site URL should parse");
        let (removed_site, preserved_site) = cache.execute(|connection| {
            let removed_site = SiteRepository
                .upsert_self_hosted_site(
                    connection,
                    &SelfHostedSite {
                        url: removed_url.url(),
                        api_root: "https://removed.example.com/wp-json".to_string(),
                    },
                )
                .expect("removed site should be inserted")
                .db_site;
            let preserved_site = SiteRepository
                .upsert_self_hosted_site(
                    connection,
                    &SelfHostedSite {
                        url: "https://preserved.example.com".to_string(),
                        api_root: "https://preserved.example.com/wp-json".to_string(),
                    },
                )
                .expect("preserved site should be inserted")
                .db_site;
            (removed_site, preserved_site)
        });

        assert_site_removal_cascades(&cache, &removed_site, &preserved_site, || {
            cache.remove_self_hosted_site(Arc::new(removed_url))
        });
    }

    #[test]
    fn direct_api_remove_wordpress_com_site_cascades_with_foreign_keys_enabled() {
        let cache = cache_from_connection_with_foreign_keys_disabled();
        let removed_site_id = WpComSiteId(123456);
        let (removed_site, preserved_site) = cache.execute(|connection| {
            let removed_site = SiteRepository
                .upsert_wordpress_com_site(
                    connection,
                    &WordPressComSite {
                        site_id: removed_site_id,
                    },
                )
                .expect("WordPress.com site should be inserted")
                .db_site;
            let preserved_site = SiteRepository
                .upsert_self_hosted_site(
                    connection,
                    &SelfHostedSite {
                        url: "https://preserved.example.com".to_string(),
                        api_root: "https://preserved.example.com/wp-json".to_string(),
                    },
                )
                .expect("preserved site should be inserted")
                .db_site;
            (removed_site, preserved_site)
        });

        assert_site_removal_cascades(&cache, &removed_site, &preserved_site, || {
            cache.remove_wordpress_com_site(removed_site_id)
        });
    }

    #[test]
    fn test_migration_works() {
        let cache = WpApiCache::new(None).unwrap();
        assert!(foreign_keys_are_enabled(&cache));
        let migrations_run = cache.perform_migrations().unwrap();
        assert_eq!(migrations_run, MIGRATION_QUERIES.len() as i64);
    }

    #[test]
    fn test_try_from_enables_foreign_keys() {
        let connection = Connection::open_in_memory().expect("in-memory connection should open");
        connection
            .pragma_update(None, "foreign_keys", "OFF")
            .expect("foreign keys should be disabled before cache creation");

        let cache = WpApiCache::try_from(connection).expect("cache should be created");

        assert!(foreign_keys_are_enabled(&cache));
    }

    #[test]
    fn test_migration_numbering_should_be_sequential() {
        let connection = Connection::open_in_memory().unwrap();
        let mut migration_manager = MigrationManager::new(&connection).unwrap();

        // Create migrations table and run first migration manually
        migration_manager.create_migrations_table().unwrap();
        migration_manager.insert_migration(1).unwrap();

        migration_manager
            .perform_migrations()
            .expect("Migrations should succeed");

        // Verify migration IDs are sequential
        let mut stmt = connection
            .prepare("SELECT migration_id FROM _migrations ORDER BY migration_id")
            .unwrap();
        let migration_ids: Vec<i64> = stmt
            .query_map([], |row| row.get::<_, i64>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        // Verify migration IDs are sequential and complete
        let expected_ids: Vec<i64> = (1..=MIGRATION_QUERIES.len() as i64).collect();
        assert_eq!(
            migration_ids,
            expected_ids,
            "Migration IDs should be sequential from 1 to {}",
            MIGRATION_QUERIES.len()
        );
    }

    #[test]
    fn test_bundled_sqlite_version_supports_returning() {
        let conn = Connection::open_in_memory().unwrap();
        let version = get_sqlite_version(&conn).expect("Failed to get SQLite version");

        println!("Bundled SQLite version: {}", version);

        assert!(
            does_sqlite_support_returning(&version),
            "SQLite version {} is too old for RETURNING clause (need 3.35.0+)",
            version
        );
    }

    #[test]
    fn test_does_sqlite_support_returning() {
        // Supported versions
        assert!(does_sqlite_support_returning("3.35.0"));
        assert!(does_sqlite_support_returning("3.45.0"));
        assert!(does_sqlite_support_returning("4.0.0"));

        // Unsupported versions
        assert!(!does_sqlite_support_returning("3.34.0"));
        assert!(!does_sqlite_support_returning("2.8.17"));

        // Invalid version strings
        assert!(!does_sqlite_support_returning("invalid"));
        assert!(!does_sqlite_support_returning(""));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_concurrent_migrations_race_condition() {
        let db_path = "/tmp/wp_cache_race_test.db";
        let _ = std::fs::remove_file(db_path);

        let num_tasks = 10;
        let mut tasks = vec![];

        for task_id in 0..num_tasks {
            let task = tokio::spawn(async move {
                let cache = WpApiCache::new(Some(db_path.to_string())).unwrap();
                let result = cache.perform_migrations();

                (task_id, result)
            });

            tasks.push(task);
        }

        let results: Vec<(usize, Result<i64, SqliteDbError>)> = futures::future::join_all(tasks)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        let mut error_count = 0;

        for (task_id, result) in &results {
            match result {
                Ok(migrations_run) => {
                    println!(
                        "Task {} succeeded, ran {} migrations",
                        task_id, migrations_run
                    );
                }
                Err(e) => {
                    println!("Task {} failed with error: {}", task_id, e);
                    error_count += 1;
                }
            }
        }

        let migrations_run_counts: Vec<i64> = results
            .iter()
            .filter_map(|(_, result)| result.as_ref().ok())
            .copied()
            .collect();
        let full_migration_count = migrations_run_counts
            .iter()
            .filter(|&&count| count == MIGRATION_QUERIES.len() as i64)
            .count();
        let zero_migration_count = migrations_run_counts
            .iter()
            .filter(|&&count| count == 0)
            .count();

        assert_eq!(
            full_migration_count,
            1,
            "Expected exactly one task to run all {} migrations, but {} tasks did",
            MIGRATION_QUERIES.len(),
            full_migration_count
        );
        assert_eq!(
            zero_migration_count,
            migrations_run_counts.len() - 1,
            "Expected {} tasks to run zero migrations, but {} tasks did",
            migrations_run_counts.len() - 1,
            zero_migration_count
        );

        let final_cache = WpApiCache::new(Some(db_path.to_string())).unwrap();
        let migration_count = final_cache.execute(|conn| {
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM _migrations").unwrap();
            stmt.query_row([], |row| row.get::<_, i64>(0))
        });

        assert_eq!(migration_count.unwrap(), MIGRATION_QUERIES.len() as i64);
        assert_eq!(
            error_count, 0,
            "Race condition detected: {} tasks failed",
            error_count
        );
    }

    #[test]
    fn test_migration_returns_error_when_index_exceeds_available_migrations() {
        let connection = Connection::open_in_memory().unwrap();
        let mut migration_manager = MigrationManager::new(&connection).unwrap();

        migration_manager.create_migrations_table().unwrap();

        // Insert a migration ID beyond the number of available migrations
        let beyond_max = MIGRATION_QUERIES.len() as i64 + 1;
        migration_manager.insert_migration(beyond_max).unwrap();

        let result = migration_manager.perform_migrations();
        assert!(
            result.is_err(),
            "Expected error when migration index exceeds available migrations"
        );
    }

    #[test]
    fn test_urls_eq_matches_across_trailing_slash() {
        let conn = Connection::open_in_memory().unwrap();
        register_url_functions(&conn).unwrap();

        for (a, b) in [
            ("https://example.com", "https://example.com/"),
            ("https://example.com/", "https://example.com"),
            ("https://example.com", "https://example.com"),
        ] {
            let result: bool = conn
                .query_row("SELECT urls_eq(?, ?)", [a, b], |row| row.get(0))
                .unwrap();
            assert!(result, "{a:?} should equal {b:?}");
        }
    }

    #[test]
    fn test_urls_eq_is_case_insensitive_in_scheme_and_host() {
        let conn = Connection::open_in_memory().unwrap();
        register_url_functions(&conn).unwrap();

        let result: bool = conn
            .query_row(
                "SELECT urls_eq(?, ?)",
                ["http://localhost", "HTTP://LOCALHOST"],
                |row| row.get(0),
            )
            .unwrap();
        assert!(result);
    }

    #[test]
    fn test_urls_eq_returns_false_for_different_hosts() {
        let conn = Connection::open_in_memory().unwrap();
        register_url_functions(&conn).unwrap();

        let result: bool = conn
            .query_row(
                "SELECT urls_eq(?, ?)",
                ["https://example.com", "https://other.example.com"],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!result);
    }

    #[test]
    fn test_urls_eq_falls_back_to_string_equality_for_unparseable_input() {
        let conn = Connection::open_in_memory().unwrap();
        register_url_functions(&conn).unwrap();

        let same: bool = conn
            .query_row("SELECT urls_eq(?, ?)", ["not a url", "not a url"], |row| {
                row.get(0)
            })
            .unwrap();
        assert!(same, "identical unparseable strings should match");

        let different: bool = conn
            .query_row(
                "SELECT urls_eq(?, ?)",
                ["not a url", "also not a url"],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!different, "distinct unparseable strings should not match");
    }
}
