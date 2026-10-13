use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use wp_mobile_sqlite::ColumnIndex;

uniffi::custom_newtype!(PostWorkingCopyId, i64);

/// Identifies a post working copy within the local content database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PostWorkingCopyId(pub i64);

impl ToSql for PostWorkingCopyId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0))
    }
}

impl FromSql for PostWorkingCopyId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        i64::column_result(value).map(PostWorkingCopyId)
    }
}

/// Column indexes for post_working_copies table.
/// These must match the order of columns in the CREATE TABLE statement.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum PostWorkingCopyColumn {
    Rowid = 0,
    Title = 1,
    Content = 2,
    CreatedAt = 3,
    UpdatedAt = 4,
}

impl ColumnIndex for PostWorkingCopyColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}

/// The fields of a post that the user edits on the device.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PostWorkingCopy {
    pub title: String,
    pub content: String,
}

/// A post working copy as stored, with its ID and timestamps.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DbPostWorkingCopy {
    pub id: PostWorkingCopyId,
    pub working_copy: PostWorkingCopy,
    /// ISO 8601 UTC timestamp of when the working copy was created.
    pub created_at: String,
    /// ISO 8601 UTC timestamp of the last save.
    pub updated_at: String,
}
