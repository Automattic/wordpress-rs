use rusqlite::hooks::Action;
use rusqlite::{Connection, Result as SqliteResult, params};
use std::sync::{Arc, Mutex};

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

#[derive(uniffi::Object)]
pub struct WpApiCache {
    inner: DBManager
}

#[uniffi::export]
impl WpApiCache {
    #[uniffi::constructor]
    pub fn new(path: Option<String>) -> Result<Self, SqliteDbError> {
        Ok(Self {
            inner: DBManager::new(&path)?
        })
    }

    pub fn perform_migrations(&self) -> Result<u64, SqliteDbError> {
        let connection: &Connection = &self.inner.connection.lock().unwrap();
        Ok(MigrationManager::new(connection)?.perform_migrations()?)
    }

    pub fn flush(&self) -> Result<(), SqliteDbError> {
        let connection: &Connection = &self.inner.connection.lock().unwrap();
        connection.execute("commit", ())?;
        Ok(())
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

static MIGRATION_QUERIES: [&str; 2] = [
    include_str!("migrations/0001-create-posts-table.sql"),
    include_str!("migrations/0002-create-users-table.sql"),
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

            self.insert_migration((index + 1) as u64)?;
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

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct UpdateHook {
    action: HookAction,
    db_name: String,
    table_name: String,
    row_id: i64,
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

        Ok(Self { connection: Mutex::new(connection) })
    }
}
