use crate::LocalContentError;
use rusqlite::Connection;

pub mod working_copies;

/// Runs SQL statements on a connection or a transaction.
///
/// Repository functions take an executor instead of a `Connection`, so they
/// get `prepare` and `execute` but none of the connection's other methods,
/// such as `transaction`.
pub trait QueryExecutor {
    /// Prepare a SQL statement for execution.
    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement<'_>, LocalContentError>;

    /// Execute a SQL statement with parameters and return the number of affected rows.
    fn execute(&self, sql: &str, params: impl rusqlite::Params)
    -> Result<usize, LocalContentError>;
}

/// A [`QueryExecutor`] that is a transaction.
///
/// Only `rusqlite::Transaction` implements it, so a repository function that
/// takes `&impl InTransaction` can't be called on a bare `Connection`.
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
