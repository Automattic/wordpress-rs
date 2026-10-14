use crate::LocalContentError;
use rusqlite::Connection;

pub mod working_copies;

/// Abstraction over database query execution.
///
/// Repository functions take an executor rather than a `Connection` so they can
/// only prepare and execute statements: they can't open transactions, change
/// pragmas, or otherwise reconfigure the connection they're handed.
pub trait QueryExecutor {
    /// Prepare a SQL statement for execution.
    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement<'_>, LocalContentError>;

    /// Execute a SQL statement with parameters and return the number of affected rows.
    fn execute(&self, sql: &str, params: impl rusqlite::Params)
    -> Result<usize, LocalContentError>;
}

/// Trait for types that can manage database transactions.
///
/// This is separate from QueryExecutor because not all query executors can create transactions
/// (e.g., a Transaction itself cannot create nested transactions in our design).
pub trait TransactionManager: QueryExecutor {
    /// Begin a database transaction (requires mutable access).
    fn transaction(&mut self) -> Result<rusqlite::Transaction<'_>, LocalContentError>;
}

/// Marker trait indicating operations are executing within a transaction context.
///
/// This trait is only implemented by Transaction types, so an operation that takes
/// `&impl InTransaction` can't be called outside a transaction. This provides stronger
/// guarantees than accepting `&impl QueryExecutor`, which allows both Connection and
/// Transaction.
pub trait InTransaction: QueryExecutor {}

impl QueryExecutor for Connection {
    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement<'_>, LocalContentError> {
        self.prepare(sql).map_err(LocalContentError::from)
    }

    fn execute(
        &self,
        sql: &str,
        params: impl rusqlite::Params,
    ) -> Result<usize, LocalContentError> {
        self.execute(sql, params).map_err(LocalContentError::from)
    }
}

impl TransactionManager for Connection {
    fn transaction(&mut self) -> Result<rusqlite::Transaction<'_>, LocalContentError> {
        rusqlite::Connection::transaction(self).map_err(LocalContentError::from)
    }
}

impl<'conn> QueryExecutor for rusqlite::Transaction<'conn> {
    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement<'_>, LocalContentError> {
        rusqlite::Connection::prepare(self, sql).map_err(Into::into)
    }

    fn execute(
        &self,
        sql: &str,
        params: impl rusqlite::Params,
    ) -> Result<usize, LocalContentError> {
        rusqlite::Connection::execute(self, sql, params).map_err(Into::into)
    }
}

impl<'conn> InTransaction for rusqlite::Transaction<'conn> {}
