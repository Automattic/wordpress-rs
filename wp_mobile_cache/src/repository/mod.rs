use crate::{RowId, SqliteDbError, mappings::InsertIntoDb};
use rusqlite::Connection;

pub mod posts;

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
/// Types implementing this trait can be persisted to the database.
/// They must specify their table name and implement serialization.
pub trait DbEntity: InsertIntoDb {
    /// The name of the database table for this entity.
    const TABLE_NAME: &'static str;
}

/// Repository trait providing common CRUD operations for database entities.
///
/// This trait provides default implementations for common operations. Concrete
/// repositories can add type-specific methods as needed.
///
/// # Example
///
/// ```ignore
/// struct PostRepository;
///
/// impl Repository for PostRepository {
///     type Entity = DbAnyPostWithEditContext;
/// }
///
/// impl PostRepository {
///     pub fn select_by_post_id(&self, executor: &impl QueryExecutor, post_id: PostId)
///         -> Result<DbAnyPostWithEditContext, SqliteDbError> {
///         // Custom implementation
///     }
/// }
/// ```
pub trait Repository {
    /// The database entity type this repository manages.
    type Entity: DbEntity;

    /// Insert a single entity into the database.
    ///
    /// Returns the rowid of the newly inserted row.
    fn insert(
        &self,
        executor: &impl QueryExecutor,
        item: &Self::Entity,
        site: &crate::DbSite,
    ) -> Result<RowId, SqliteDbError> {
        item.insert_into_db(executor, site)
    }

    /// Insert multiple entities in a single transaction.
    ///
    /// Returns a vector of rowids for the inserted entities in the same order.
    /// If any insert fails, the entire transaction is rolled back.
    fn insert_batch(
        &self,
        transaction_manager: &mut impl TransactionManager,
        items: &[Self::Entity],
        site: &crate::DbSite,
    ) -> Result<Vec<RowId>, SqliteDbError> {
        let tx = transaction_manager.transaction()?;
        let mut rowids = Vec::with_capacity(items.len());

        for item in items {
            let rowid = InsertIntoDb::insert_into_db(item, &tx, site)?;
            rowids.push(rowid);
        }

        tx.commit().map_err(SqliteDbError::from)?;
        Ok(rowids)
    }
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

    impl InsertIntoDb for TestEntity {
        fn insert_into_db(
            &self,
            executor: &impl QueryExecutor,
            _site: &crate::DbSite,
        ) -> Result<RowId, SqliteDbError> {
            executor.execute("INSERT INTO test_table (value) VALUES (?)", [&self.value])?;
            Ok(QueryExecutor::last_insert_rowid(executor))
        }
    }

    impl DbEntity for TestEntity {
        const TABLE_NAME: &'static str = "test_table";
    }

    #[allow(dead_code)]
    struct TestRepository;

    impl Repository for TestRepository {
        type Entity = TestEntity;
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
