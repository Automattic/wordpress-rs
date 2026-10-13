use rusqlite::Connection;
use std::sync::Mutex;

use crate::functions::register_url_functions;

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error("Foreign key enforcement could not be enabled on the connection")]
    ForeignKeysUnavailable,
}

/// A SQLite connection guarded by a mutex.
///
/// Every connection it holds has foreign key enforcement on and the custom SQL
/// functions from [`crate::functions`] registered.
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
}
