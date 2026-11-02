//! The local content database, which stores the posts and pages the user
//! creates or edits on the device as working copies.
//!
//! A working copy is the user's version of a post or page. Unlike the API
//! cache, it can't be fetched again, so it stays until it's deleted on purpose.
//! Nothing in this crate sends anything to the server. See
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
    #[error(
        "The working copy edits remote post {}, so it can't be pushed to {}",
        edited_post_id.0,
        pushed_post_id.0
    )]
    PushedToAnotherRemotePost {
        edited_post_id: PostId,
        pushed_post_id: PostId,
    },
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

/// The local content database.
///
/// Open it with `new`, then call `perform_migrations` before reading or
/// writing anything.
#[derive(uniffi::Object)]
pub struct WpLocalContentStore {
    connection: DbConnection,
}

#[uniffi::export]
impl WpLocalContentStore {
    /// Open the local content database file at `path`, creating it if it doesn't exist.
    ///
    /// `path` must be an absolute path to a file. Fails with
    /// `InvalidDatabasePath` for anything else, such as a relative path,
    /// `:memory:` or a `file:` URI. In-memory and temporary databases are
    /// rejected because their content is gone when the process exits.
    #[uniffi::constructor]
    pub fn new(path: String) -> Result<Self, LocalContentError> {
        Ok(Self {
            connection: DbConnection::open_file(&path)?,
        })
    }

    /// Create or update the database's tables to the current schema, and
    /// return how many migrations ran.
    ///
    /// Call it after opening the store, before reading or writing anything.
    pub fn perform_migrations(&self) -> Result<i64, LocalContentError> {
        self.execute(|connection| {
            let mut mgr = MigrationManager::new(connection, &MIGRATIONS)?;
            Ok(mgr.perform_migrations_with_foreign_keys_off()?)
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
///
/// They run with foreign key enforcement off, and every foreign key is checked
/// before they commit. Dropping a table therefore leaves the rows that
/// reference it alone. This lets a migration rebuild a table that others
/// reference, such as `working_copies`, the way SQLite documents: create the
/// new table, copy the rows into it, drop the old one, then rename the new one
/// to the old name.
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
