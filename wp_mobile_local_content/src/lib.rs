//! On-device storage for content the user creates or edits.
//!
//! This database is separate from the API cache: the cache can be cleared and
//! fetched again, local content can't. See
//! `Documentation/local-content-principles.md`.

use rusqlite::Connection;
use wp_mobile_sqlite::{ConnectionError, DbConnection, MigrationManager};

use crate::db_types::post_working_copy::PostWorkingCopyId;

pub mod db_types;
pub mod repository;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum LocalContentError {
    #[error("SQLite error: {message}")]
    Sqlite { message: String },
    #[error("Foreign key enforcement could not be enabled on the connection")]
    ForeignKeysUnavailable,
    #[error("No post working copy with id {}", id.0)]
    PostWorkingCopyNotFound { id: PostWorkingCopyId },
}

impl From<rusqlite::Error> for LocalContentError {
    fn from(err: rusqlite::Error) -> Self {
        LocalContentError::Sqlite {
            message: err.to_string(),
        }
    }
}

impl From<ConnectionError> for LocalContentError {
    fn from(err: ConnectionError) -> Self {
        match err {
            ConnectionError::Sqlite(err) => err.into(),
            ConnectionError::ForeignKeysUnavailable => LocalContentError::ForeignKeysUnavailable,
        }
    }
}

/// The database that holds local content.
#[derive(uniffi::Object)]
pub struct WpLocalContentStore {
    connection: DbConnection,
}

#[uniffi::export]
impl WpLocalContentStore {
    /// Open the local content database at `path`, or an in-memory database when
    /// `path` is `None`.
    #[uniffi::constructor]
    pub fn new(path: Option<String>) -> Result<Self, LocalContentError> {
        Ok(Self {
            connection: DbConnection::open(path.as_deref())?,
        })
    }

    pub fn perform_migrations(&self) -> Result<i64, LocalContentError> {
        self.execute(|connection| {
            let mut mgr = MigrationManager::new(connection, &MIGRATIONS)?;
            Ok(mgr.perform_migrations()?)
        })
    }
}

impl WpLocalContentStore {
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

/// The local content migrations, in the order [`MigrationManager`] applies them.
pub static MIGRATIONS: [&str; 1] = [include_str!(
    "../migrations/0001-create-post-working-copies-table.sql"
)];

uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_works() {
        let store = WpLocalContentStore::new(None).expect("Opening the store should succeed");
        let migrations_run = store
            .perform_migrations()
            .expect("Migrations should succeed");
        assert_eq!(migrations_run, MIGRATIONS.len() as i64);
    }
}
