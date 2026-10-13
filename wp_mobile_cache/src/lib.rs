use rusqlite::Connection;
use rusqlite::hooks::Action;
use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput};
use std::sync::Arc;

use crate::repository::entity_state::EntityStateRepository;
use crate::repository::list_metadata::ListMetadataRepository;
use crate::repository::sites::SiteRepository;
use wp_api::parsed_url::ParsedUrl;
use wp_api::wp_com::WpComSiteId;
use wp_mobile_sqlite::{ConnectionError, DbConnection, MigrationManager};

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
    ForeignKeysUnavailable,
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
            SqliteDbError::ForeignKeysUnavailable => {
                write!(
                    f,
                    "Foreign key enforcement could not be enabled on the connection"
                )
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

impl From<ConnectionError> for SqliteDbError {
    fn from(err: ConnectionError) -> Self {
        match err {
            ConnectionError::Sqlite(err) => err.into(),
            ConnectionError::ForeignKeysUnavailable => SqliteDbError::ForeignKeysUnavailable,
        }
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
    connection: DbConnection,
}

#[uniffi::export]
impl WpApiCache {
    #[uniffi::constructor]
    pub fn new(path: Option<String>) -> Result<Self, SqliteDbError> {
        Ok(Self {
            connection: DbConnection::open(path.as_deref())?,
        })
    }

    pub fn perform_migrations(&self) -> Result<i64, SqliteDbError> {
        self.execute(|connection| {
            let mut mgr = MigrationManager::new(connection, &MIGRATIONS)?;
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
            if let Err(e) = connection.update_hook(Some(
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
            )) {
                log::warn!("Failed to register update hook: {}", e);
            }
        });
    }

    pub fn stop_listening_for_updates(&self) {
        self.execute(|connection| {
            if let Err(e) = connection.update_hook(None::<fn(Action, &str, &str, i64)>) {
                log::warn!("Failed to clear update hook: {}", e);
            }
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
    /// See [`DbConnection::execute`] for the locking rules.
    pub fn execute<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Connection) -> R,
    {
        self.connection.execute(f)
    }
}

impl TryFrom<Connection> for WpApiCache {
    type Error = SqliteDbError;

    /// Create a WpApiCache from an existing connection.
    ///
    /// This is typically used in tests to create a cache from an already-migrated
    /// in-memory database connection.
    fn try_from(connection: Connection) -> Result<Self, Self::Error> {
        Ok(Self {
            connection: DbConnection::try_from(connection)?,
        })
    }
}

/// The cache's migrations, in the order [`MigrationManager`] applies them.
pub static MIGRATIONS: [&str; 14] = [
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

uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::get_table_column_names;

    fn foreign_keys_enabled(conn: &Connection) -> bool {
        conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .expect("Failed to read foreign_keys pragma")
    }

    /// A connection with enforcement explicitly turned off, standing in for a
    /// SQLite build whose default leaves it off.
    fn connection_without_foreign_keys() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory database");
        conn.pragma_update(None, "foreign_keys", false)
            .expect("Failed to disable foreign_keys");
        assert!(!foreign_keys_enabled(&conn));
        conn
    }

    #[test]
    fn test_cache_constructor_enables_foreign_keys() {
        let cache = WpApiCache::new(None).expect("Cache creation should succeed");
        assert!(
            cache.execute(|conn| foreign_keys_enabled(conn)),
            "WpApiCache::new must enable foreign key enforcement"
        );
    }

    #[test]
    fn test_try_from_enables_foreign_keys_on_a_connection_that_had_them_off() {
        let cache = WpApiCache::try_from(connection_without_foreign_keys())
            .expect("Cache creation should succeed");
        assert!(
            cache.execute(|conn| foreign_keys_enabled(conn)),
            "WpApiCache::try_from must enable foreign key enforcement rather than \
             inherit whatever the incoming connection had"
        );
    }

    /// `SiteRepository::delete_site` deletes only the `db_sites` row and relies on
    /// `ON DELETE CASCADE` for everything else, so a child table that reaches
    /// `db_sites` without that clause leaks its rows on every site removal.
    ///
    /// `entity_state` is the deliberate exception: it carries no constraint and is
    /// cleared by an explicit `DELETE` in `delete_site`.
    #[test]
    fn test_every_table_referencing_db_sites_cascades_on_delete() {
        const NO_CASCADE_CONSTRAINT: [DbTable; 1] = [DbTable::EntityState];

        let conn = Connection::open_in_memory().expect("Failed to open in-memory database");
        let mut migration_manager =
            MigrationManager::new(&conn, &MIGRATIONS).expect("Failed to create MigrationManager");
        migration_manager
            .perform_migrations()
            .expect("All migrations should succeed");

        // Read the table list from the schema rather than from `DbTable`, so a
        // table added by a future migration is covered whether or not it is
        // also exposed as a `DbTable` variant.
        let exempt: Vec<&str> = NO_CASCADE_CONSTRAINT
            .iter()
            .map(|table| table.table_name())
            .collect();
        let tables: Vec<String> = conn
            .prepare(
                "SELECT name FROM sqlite_master \
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND name != '_migrations'",
            )
            .expect("Failed to prepare sqlite_master query")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("Failed to execute sqlite_master query")
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to collect table names");

        let mut checked = 0;
        for table in tables {
            let has_db_site_id = get_table_column_names(&conn, &table)
                .iter()
                .any(|column| column == "db_site_id");
            if !has_db_site_id || exempt.contains(&table.as_str()) {
                continue;
            }

            // (referenced table, on_delete) for each outgoing foreign key
            let foreign_keys: Vec<(String, String)> = conn
                .prepare(&format!("PRAGMA foreign_key_list({})", table))
                .expect("Failed to prepare PRAGMA query")
                .query_map([], |row| Ok((row.get(2)?, row.get(6)?)))
                .expect("Failed to execute PRAGMA query")
                .collect::<Result<Vec<_>, _>>()
                .expect("Failed to collect foreign keys");

            assert!(
                foreign_keys.contains(&(
                    DbTable::DbSites.table_name().to_string(),
                    "CASCADE".to_string()
                )),
                "{} has a db_site_id column but no ON DELETE CASCADE foreign key to {}; \
                 its rows would survive delete_site. Foreign keys found: {:?}",
                table,
                DbTable::DbSites,
                foreign_keys
            );
            checked += 1;
        }

        assert!(checked > 0, "No tables were checked");
    }

    #[test]
    fn test_migration_works() {
        let cache = WpApiCache::new(None).unwrap();
        let migrations_run = cache.perform_migrations().unwrap();
        assert_eq!(migrations_run, MIGRATIONS.len() as i64);
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
}
