use rusqlite::Row;

pub mod helpers;
pub mod posts;
pub mod term_relationships;

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
