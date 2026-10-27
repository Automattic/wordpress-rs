use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::sync::Mutex;

use crate::functions::register_url_functions;

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error("Foreign key enforcement could not be enabled on the connection")]
    ForeignKeysUnavailable,
    #[error("Expected an absolute file path, got {0:?}")]
    NotAnAbsoluteFilePath(String),
}

/// A SQLite connection guarded by a mutex.
///
/// Every connection it holds has foreign key enforcement on and the `urls_eq`
/// SQL function registered.
pub struct DbConnection {
    connection: Mutex<Connection>,
}

impl DbConnection {
    /// Open the database at `path`, or an in-memory database when `path` is `None`.
    pub fn open(path: Option<&str>) -> Result<Self, ConnectionError> {
        let connection = match path {
            Some(path) => Connection::open(path)?,
            None => Connection::open_in_memory()?,
        };
        Self::try_from(connection)
    }

    /// Open the database file at `path`, creating it if it doesn't exist.
    ///
    /// Unlike [`DbConnection::open`], this only ever opens a file on disk:
    /// - `path` must be absolute, so `":memory:"`, the empty string (SQLite's
    ///   temporary database) and paths relative to the process's working
    ///   directory are rejected.
    /// - URI filenames are not interpreted, so a `file:` URI or a `?mode=memory`
    ///   query can't select an in-memory database.
    pub fn open_file(path: &str) -> Result<Self, ConnectionError> {
        if !Path::new(path).is_absolute() {
            return Err(ConnectionError::NotAnAbsoluteFilePath(path.to_string()));
        }
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        Self::try_from(Connection::open_with_flags(path, flags)?)
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
    /// let post = db.execute(|conn| {
    ///     repo.select_by_id(conn, post_id)
    /// })?;
    ///
    /// // Write operation
    /// db.execute(|conn| {
    ///     repo.upsert(conn, &site, &post)
    /// })?;
    /// ```
    ///
    /// # Preventing Deadlocks
    ///
    /// **Safe** - Lock is released between calls:
    /// ```ignore
    /// let data = db.execute(|conn| read(conn))?;
    /// db.execute(|conn| write(conn, data))?;  // ✅ OK
    /// ```
    ///
    /// **Unsafe** - Nested calls, on the same database or across two databases:
    /// ```ignore
    /// db.execute(|conn| {
    ///     other_db.execute(|conn2| { ... })  // ⚠️ Deadlock if another caller nests them in reverse
    /// })?;
    /// ```
    pub fn execute<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Connection) -> R,
    {
        let mut conn = self.connection.lock().unwrap();
        f(&mut conn)
    }
}

impl TryFrom<Connection> for DbConnection {
    type Error = ConnectionError;

    /// Wrap an existing connection, configuring it the same way [`DbConnection::open`] does.
    fn try_from(connection: Connection) -> Result<Self, Self::Error> {
        prepare_connection(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
}

/// Apply the per-connection settings every database relies on.
///
/// Both steps are idempotent, so running them on an already prepared
/// connection is harmless.
pub(crate) fn prepare_connection(conn: &Connection) -> Result<(), ConnectionError> {
    enable_foreign_keys(conn)?;
    register_url_functions(conn)?;
    Ok(())
}

/// Turn on foreign key enforcement for `conn` and confirm it took effect.
///
/// Schemas rely on `ON DELETE CASCADE` to clear the rows belonging to a
/// deleted parent. SQLite honours those clauses only while `foreign_keys` is
/// on — a per-connection setting whose default is not guaranteed — and when it
/// is off the delete still succeeds and silently leaves the children behind.
///
/// The value is read back rather than assumed because `PRAGMA foreign_keys` is
/// a no-op inside an open transaction and reports no error when it is ignored.
fn enable_foreign_keys(conn: &Connection) -> Result<(), ConnectionError> {
    conn.pragma_update(None, "foreign_keys", true)?;

    let enabled: bool = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
    if enabled {
        Ok(())
    } else {
        Err(ConnectionError::ForeignKeysUnavailable)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn foreign_keys_enabled(conn: &Connection) -> bool {
        conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .expect("Failed to read foreign_keys pragma")
    }

    /// A connection with enforcement explicitly turned off, standing in for a
    /// SQLite build whose default leaves it off.
    pub(crate) fn connection_without_foreign_keys() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory database");
        conn.pragma_update(None, "foreign_keys", false)
            .expect("Failed to disable foreign_keys");
        assert!(!foreign_keys_enabled(&conn));
        conn
    }

    #[test]
    fn test_open_enables_foreign_keys() {
        let db = DbConnection::open(None).expect("Opening an in-memory database should succeed");
        assert!(
            db.execute(|conn| foreign_keys_enabled(conn)),
            "DbConnection::open must enable foreign key enforcement"
        );
    }

    #[test]
    fn test_try_from_enables_foreign_keys_on_a_connection_that_had_them_off() {
        let db = DbConnection::try_from(connection_without_foreign_keys())
            .expect("Wrapping the connection should succeed");
        assert!(
            db.execute(|conn| foreign_keys_enabled(conn)),
            "DbConnection::try_from must enable foreign key enforcement rather than \
             inherit whatever the incoming connection had"
        );
    }

    #[test]
    fn test_open_file_rejects_paths_that_are_not_absolute_files() {
        for path in [":memory:", "", "relative.db", "file:local.db?mode=memory"] {
            let result = DbConnection::open_file(path);
            assert!(
                matches!(&result, Err(ConnectionError::NotAnAbsoluteFilePath(p)) if p == path),
                "{path:?} should be rejected, got {:?}",
                result.err()
            );
        }
    }

    #[test]
    fn test_open_file_treats_uri_syntax_as_a_literal_file_name() {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let path = dir.path().join("local.db?mode=memory");
        let path = path.to_str().expect("Temp path should be UTF-8");

        let db = DbConnection::open_file(path).expect("Opening the file should succeed");
        db.execute(|conn| conn.execute("CREATE TABLE t (id INTEGER)", ()))
            .expect("Creating a table should succeed");

        assert!(
            Path::new(path).is_file(),
            "The query string should be part of the file name, not an in-memory database"
        );
    }

    #[test]
    fn test_open_file_enables_foreign_keys() {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let path = dir.path().join("local.db");
        let db = DbConnection::open_file(path.to_str().expect("Temp path should be UTF-8"))
            .expect("Opening the file should succeed");
        assert!(
            db.execute(|conn| foreign_keys_enabled(conn)),
            "DbConnection::open_file must enable foreign key enforcement"
        );
    }
}
