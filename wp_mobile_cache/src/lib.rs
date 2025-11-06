use rusqlite::hooks::Action;
use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput};
use rusqlite::{Connection, Result as SqliteResult, params};
use std::sync::{Arc, Mutex};

pub mod context;
pub mod db_types;
pub mod entity_id;
pub mod repository;
pub mod term_relationships;

#[cfg(any(test, feature = "test-helpers"))]
pub mod test_fixtures;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum SqliteDbError {
    SqliteError(String),
}

impl std::fmt::Display for SqliteDbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SqliteDbError::SqliteError(message) => write!(f, "SqliteDbError: message={}", message),
        }
    }
}

impl From<rusqlite::Error> for SqliteDbError {
    fn from(err: rusqlite::Error) -> Self {
        SqliteDbError::SqliteError(err.to_string())
    }
}

/// Represents a database row ID (autoincrement field).
/// SQLite rowids are guaranteed to be non-negative, so we use u64.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RowId(pub u64);

impl ToSql for RowId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0 as i64))
    }
}

impl FromSql for RowId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> FromSqlResult<Self> {
        i64::column_result(value).map(|i| {
            debug_assert!(i >= 0, "RowId should be non-negative, got: {}", i);
            RowId(i as u64)
        })
    }
}

impl From<i64> for RowId {
    fn from(value: i64) -> Self {
        debug_assert!(value >= 0, "RowId should be non-negative, got: {}", value);
        RowId(value as u64)
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
        row_id.0 as i64
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

    pub fn perform_migrations(&self) -> Result<u64, SqliteDbError> {
        let connection: &Connection = &self.inner.connection.lock().unwrap();
        Ok(MigrationManager::new(connection)?.perform_migrations()?)
    }

    pub fn start_listening_for_updates(&self, delegate: Arc<dyn DatabaseDelegate>) {
        let connection: &Connection = &self.inner.connection.lock().unwrap();
        connection.update_hook(Some(
            move |action: Action, db_name: &str, table_name: &str, row_id: i64| {
                let hook_data = UpdateHook {
                    action: action.into(),
                    db_name: db_name.to_string(),
                    table_name: table_name.to_string(),
                    row_id,
                };

                delegate.did_update(hook_data);
            },
        ));
    }

    pub fn stop_listening_for_updates(&self) {
        let connection: &Connection = &self.inner.connection.lock().unwrap();
        connection.update_hook(None::<fn(Action, &str, &str, i64)>);
    }
}

impl WpApiCache {
    /// Get access to the database connection
    ///
    /// Returns a MutexGuard that implements both QueryExecutor and TransactionManager.
    /// The connection is automatically unlocked when the guard is dropped.
    pub fn connection(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.inner.connection.lock().unwrap()
    }
}

impl From<Connection> for WpApiCache {
    fn from(connection: Connection) -> Self {
        Self {
            inner: DBManager {
                connection: Mutex::new(connection),
            },
        }
    }
}

static MIGRATION_QUERIES: [&str; 6] = [
    include_str!("../migrations/0001-create-sites-table.sql"),
    include_str!("../migrations/0002-create-posts-table.sql"),
    include_str!("../migrations/0003-create-term-relationships.sql"),
    include_str!("../migrations/0004-create-posts-view-context-table.sql"),
    include_str!("../migrations/0005-create-posts-embed-context-table.sql"),
    include_str!("../migrations/0006-create-self-hosted-sites-table.sql"),
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

    pub fn perform_migrations(&mut self) -> SqliteResult<u64> {
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
            self.insert_migration((next_migration_id + index + 1) as u64)?;
        }

        Ok(MIGRATION_QUERIES[next_migration_id..].len() as u64)
    }

    pub fn create_migrations_table(&self) -> SqliteResult<()> {
        self.connection.execute(
            "CREATE TABLE _migrations (migration_id INTEGER PRIMARY KEY)",
            (),
        )?;
        Ok(())
    }

    pub fn insert_migration(&mut self, migration_id: u64) -> SqliteResult<()> {
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

// Re-export EntityId from entity_id module
pub use entity_id::EntityId;

/// Wrapper that pairs cached data with its database identity
///
/// When fetching data from the cache, we return both the data and
/// an EntityId that can be used to:
/// - Create observable entities without additional database queries
/// - Identify this specific entity in update notifications
/// - Compare entities for identity equality
///
/// This type is generic over the data type T
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullEntity<T> {
    /// The database identity of this entity
    pub entity_id: Arc<EntityId>,

    /// The cached data
    pub data: T,
}

impl<T> FullEntity<T> {
    /// Create a new FullEntity pairing data with its identity
    pub fn new(entity_id: Arc<EntityId>, data: T) -> Self {
        Self { entity_id, data }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct UpdateHook {
    pub action: HookAction,
    pub db_name: String,
    pub table_name: String,
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
    connection: Mutex<Connection>,
}

impl DBManager {
    pub fn new(path: &Option<String>) -> Result<Self, SqliteDbError> {
        let connection: Connection;

        if let Some(path) = path.clone() {
            connection = Connection::open(path)?;
        } else {
            connection = Connection::open_in_memory()?;
        }

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
        let migration_ids: Vec<u64> = stmt
            .query_map([], |row| row.get::<_, u64>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        // Verify migration IDs are sequential and complete
        let expected_ids: Vec<u64> = (1..=MIGRATION_QUERIES.len() as u64).collect();
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
