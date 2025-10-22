use crate::{RowId, SqliteDbError};
use rusqlite::Connection;

pub mod posts;
pub mod term_relationships;

#[cfg(test)]
mod posts_constraint_tests;
#[cfg(test)]
mod posts_multi_site_tests;
#[cfg(test)]
mod posts_transaction_tests;
#[cfg(test)]
mod term_relationships_multi_site_tests;

/// Abstraction over database query execution.
///
/// This trait decouples the repository layer from specific database implementations,
/// making it possible to use different executors (Connection, Transaction, etc.).
pub trait QueryExecutor {
    /// Prepare a SQL statement for execution.
    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement<'_>, SqliteDbError>;

    /// Execute a SQL statement with parameters and return the number of affected rows.
    fn execute(&self, sql: &str, params: impl rusqlite::Params) -> Result<usize, SqliteDbError>;

    /// Get the rowid of the last inserted row.
    fn last_insert_rowid(&self) -> RowId;
}

/// Trait for types that can manage database transactions.
///
/// This is separate from QueryExecutor because not all query executors can create transactions
/// (e.g., a Transaction itself cannot create nested transactions in our design).
pub trait TransactionManager: QueryExecutor {
    /// Begin a database transaction (requires mutable access).
    fn transaction(&mut self) -> Result<rusqlite::Transaction<'_>, SqliteDbError>;
}

impl QueryExecutor for Connection {
    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement<'_>, SqliteDbError> {
        self.prepare(sql).map_err(SqliteDbError::from)
    }

    fn execute(&self, sql: &str, params: impl rusqlite::Params) -> Result<usize, SqliteDbError> {
        self.execute(sql, params).map_err(SqliteDbError::from)
    }

    fn last_insert_rowid(&self) -> RowId {
        self.last_insert_rowid().into()
    }
}

impl TransactionManager for Connection {
    fn transaction(&mut self) -> Result<rusqlite::Transaction<'_>, SqliteDbError> {
        rusqlite::Connection::transaction(self).map_err(SqliteDbError::from)
    }
}

impl<'conn> QueryExecutor for rusqlite::Transaction<'conn> {
    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement<'_>, SqliteDbError> {
        rusqlite::Connection::prepare(self, sql).map_err(Into::into)
    }

    fn execute(&self, sql: &str, params: impl rusqlite::Params) -> Result<usize, SqliteDbError> {
        rusqlite::Connection::execute(self, sql, params).map_err(Into::into)
    }

    fn last_insert_rowid(&self) -> RowId {
        rusqlite::Connection::last_insert_rowid(self).into()
    }
}

/// Marker trait for database entities.
///
/// Types implementing this trait represent entities stored in the database.
/// They must specify their table name.
pub trait DbEntity {
    /// The name of the database table for this entity.
    const TABLE_NAME: &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mappings::TryFromDbRow;
    use rusqlite::{Connection, Row};

    #[allow(dead_code)]
    struct TestEntity {
        value: String,
    }

    impl TryFromDbRow for TestEntity {
        fn try_from_row(row: &Row) -> Result<Self, SqliteDbError> {
            Ok(TestEntity {
                value: row.get(0).map_err(SqliteDbError::from)?,
            })
        }
    }

    impl DbEntity for TestEntity {
        const TABLE_NAME: &'static str = "test_table";
    }

    #[test]
    fn test_query_executor_for_connection() {
        let conn = Connection::open_in_memory().unwrap();
        QueryExecutor::execute(
            &conn,
            "CREATE TABLE test_table (id INTEGER PRIMARY KEY, value TEXT)",
            [],
        )
        .unwrap();

        // Test prepare
        let stmt = QueryExecutor::prepare(&conn, "SELECT * FROM test_table").unwrap();
        assert!(stmt.column_count() > 0);

        // Test execute
        let affected =
            QueryExecutor::execute(&conn, "INSERT INTO test_table (value) VALUES (?)", ["test"])
                .unwrap();
        assert_eq!(affected, 1);

        // Test last_insert_rowid
        let rowid = QueryExecutor::last_insert_rowid(&conn);
        assert_eq!(rowid, RowId(1));
    }
}
