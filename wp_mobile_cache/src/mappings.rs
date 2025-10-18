use crate::SqliteDbError;
use rusqlite::Row;

pub mod helpers;
pub mod posts;

/// Trait for types that can be used as column indexes.
/// Implemented by column enum types to provide type-safe column access.
pub trait ColumnIndex {
    fn as_index(&self) -> usize;
}

/// Extension trait for `Row` that provides convenient column access.
pub trait RowExt {
    /// Get a value from a column using a column enum.
    fn get_column<T, C>(&self, column: C) -> rusqlite::Result<T>
    where
        C: ColumnIndex,
        T: rusqlite::types::FromSql;
}

impl RowExt for Row<'_> {
    fn get_column<T, C>(&self, column: C) -> rusqlite::Result<T>
    where
        C: ColumnIndex,
        T: rusqlite::types::FromSql,
    {
        self.get(column.as_index())
    }
}

/// Trait for types that can be constructed from a SQLite row.
/// Similar to `TryFrom<&Row>` but with our custom error type.
pub trait TryFromDbRow: Sized {
    fn try_from_row(row: &Row) -> Result<Self, SqliteDbError>;
}

/// Trait for types that can be inserted into the database.
/// Returns the rowid of the inserted row on success.
pub trait InsertIntoDb {
    fn insert_into_db(
        &self,
        executor: &impl crate::repository::QueryExecutor,
        site: &crate::DbSite,
    ) -> Result<crate::RowId, SqliteDbError>;
}
