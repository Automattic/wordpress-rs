//! On-device storage for content the user creates or edits.
//!
//! This database is separate from the API cache: the cache can be cleared and
//! fetched again, local content can't. See
//! `Documentation/local-content-principles.md`.

use rusqlite::Connection;
use wp_api::posts::PostId;
use wp_mobile_sqlite::{ConnectionError, DbConnection, MigrationManager};

use crate::db_types::working_copy::WorkingCopyId;

pub mod db_types;
pub mod remote;
pub mod repository;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum LocalContentError {
    #[error("SQLite error: {err_message}")]
    Sqlite { err_message: String },
    #[error("Foreign key enforcement could not be enabled on the connection")]
    ForeignKeysUnavailable,
    #[error("No working copy with id {}", id.0)]
    WorkingCopyNotFound { id: WorkingCopyId },
    #[error("The local content database needs an absolute file path, got {path:?}")]
    InvalidDatabasePath { path: String },
    #[error("Remote post {} already has a working copy", post_id.0)]
    RemotePostAlreadyHasWorkingCopy { post_id: PostId },
}

impl From<rusqlite::Error> for LocalContentError {
    fn from(err: rusqlite::Error) -> Self {
        LocalContentError::Sqlite {
            err_message: err.to_string(),
        }
    }
}

impl From<ConnectionError> for LocalContentError {
    fn from(err: ConnectionError) -> Self {
        match err {
            ConnectionError::Sqlite(err) => err.into(),
            ConnectionError::ForeignKeysUnavailable => LocalContentError::ForeignKeysUnavailable,
            ConnectionError::NotAnAbsoluteFilePath(path) => {
                LocalContentError::InvalidDatabasePath { path }
            }
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
    /// Open the local content database file at `path`, creating it if it doesn't exist.
    ///
    /// `path` must be an absolute path to a file. In-memory and temporary
    /// databases are rejected, as are relative paths and `file:` URIs: anything
    /// stored in them is gone when the process exits, which would break the
    /// first local content principle, that user content is never lost. See
    /// `Documentation/local-content-principles.md`.
    ///
    /// Fails with `InvalidDatabasePath` if `path` isn't absolute.
    #[uniffi::constructor]
    pub fn new(path: String) -> Result<Self, LocalContentError> {
        Ok(Self {
            connection: DbConnection::open_file(&path)?,
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
    "../migrations/0001-create-working-copies-tables.sql"
)];

uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_works() {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let path = dir.path().join("local_content.db");
        let store = WpLocalContentStore::new(
            path.to_str()
                .expect("Temp path should be UTF-8")
                .to_string(),
        )
        .expect("Opening the store should succeed");
        let migrations_run = store
            .perform_migrations()
            .expect("Migrations should succeed");
        assert_eq!(migrations_run, MIGRATIONS.len() as i64);
    }

    #[test]
    fn test_new_rejects_an_in_memory_database() {
        let result = WpLocalContentStore::new(":memory:".to_string());
        assert!(
            matches!(
                &result,
                Err(LocalContentError::InvalidDatabasePath { path }) if path == ":memory:"
            ),
            "Expected InvalidDatabasePath, got {:?}",
            result.err()
        );
    }
}
