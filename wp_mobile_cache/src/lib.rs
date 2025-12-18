use rusqlite::hooks::Action;
use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput};
use rusqlite::{Connection, Result as SqliteResult, params};
use std::sync::Mutex;

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
    TableNameMismatch { expected: DbTable, actual: DbTable },
}

impl std::fmt::Display for SqliteDbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SqliteDbError::SqliteError(message) => write!(f, "SqliteDbError: message={}", message),
            SqliteDbError::TableNameMismatch { expected, actual } => {
                write!(
                    f,
                    "Table mismatch: expected '{}', but got '{}'",
                    expected.table_name(),
                    actual.table_name()
                )
            }
        }
    }
}

impl From<rusqlite::Error> for SqliteDbError {
    fn from(err: rusqlite::Error) -> Self {
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
            DbTable::SelfHostedSites => "self_hosted_sites",
            DbTable::DbSites => "db_sites",
            DbTable::TermRelationships => "term_relationships",
            DbTable::ListMetadata => "list_metadata",
            DbTable::ListMetadataItems => "list_metadata_items",
            DbTable::ListMetadataState => "list_metadata_state",
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
            "self_hosted_sites" => Ok(DbTable::SelfHostedSites),
            "db_sites" => Ok(DbTable::DbSites),
            "term_relationships" => Ok(DbTable::TermRelationships),
            "list_metadata" => Ok(DbTable::ListMetadata),
            "list_metadata_items" => Ok(DbTable::ListMetadataItems),
            "list_metadata_state" => Ok(DbTable::ListMetadataState),
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
            if let Err(e) = Self::reset_stale_fetching_states_internal(connection) {
                log::warn!("Failed to reset stale fetching states: {}", e);
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
}

impl WpApiCache {
    /// Resets stale fetching states (`FetchingFirstPage`, `FetchingNextPage`) to `Idle`.
    ///
    /// If the app terminates while a fetch is in progress, these transient states persist
    /// in the database. On next launch, this could cause perpetual loading indicators or
    /// blocked fetches. We reset them during `WpApiCache` initialization since it's
    /// typically created once at app startup.
    ///
    /// Note: `Error` states are intentionally preserved for UI feedback and debugging.
    fn reset_stale_fetching_states_internal(
        connection: &mut Connection,
    ) -> Result<usize, SqliteDbError> {
        use crate::list_metadata::ListState;

        connection
            .execute(
                "UPDATE list_metadata_state SET state = ?1 WHERE state IN (?2, ?3)",
                params![
                    ListState::Idle as i32,
                    ListState::FetchingFirstPage as i32,
                    ListState::FetchingNextPage as i32,
                ],
            )
            .map_err(SqliteDbError::from)
    }

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

impl From<Connection> for WpApiCache {
    /// Create a WpApiCache from an existing connection.
    ///
    /// This is typically used in tests to create a cache from an already-migrated
    /// in-memory database connection.
    fn from(connection: Connection) -> Self {
        Self {
            inner: DBManager {
                connection: Mutex::new(connection),
            },
        }
    }
}

static MIGRATION_QUERIES: [&str; 7] = [
    include_str!("../migrations/0001-create-sites-table.sql"),
    include_str!("../migrations/0002-create-posts-table.sql"),
    include_str!("../migrations/0003-create-term-relationships.sql"),
    include_str!("../migrations/0004-create-posts-view-context-table.sql"),
    include_str!("../migrations/0005-create-posts-embed-context-table.sql"),
    include_str!("../migrations/0006-create-self-hosted-sites-table.sql"),
    include_str!("../migrations/0007-create-list-metadata-tables.sql"),
];

pub struct MigrationManager<'a> {
    connection: &'a Connection,
}

impl<'a> MigrationManager<'a> {
    pub fn new(connection: &'a Connection) -> Result<Self, SqliteDbError> {
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
        if !self.has_migrations_table()? {
            self.create_migrations_table()?;
        }

        let next_migration_id = self.get_next_migration_index()?;
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

        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
}

uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests {
    use super::*;

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
}
