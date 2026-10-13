use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use uuid::Uuid;
use wp_mobile_sqlite::ColumnIndex;

uniffi::custom_newtype!(PostWorkingCopyId, String);

/// Identifies a post working copy: a hyphenated, lowercase v4 UUID.
///
/// Unlike the table's `rowid`, it stays unique across databases.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PostWorkingCopyId(pub String);

impl PostWorkingCopyId {
    pub(crate) fn generate() -> Self {
        Self(Uuid::new_v4().hyphenated().to_string())
    }
}

impl ToSql for PostWorkingCopyId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.as_str()))
    }
}

impl FromSql for PostWorkingCopyId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        String::column_result(value).map(PostWorkingCopyId)
    }
}

/// Column indexes for post_working_copies table.
/// These must match the order of columns in the CREATE TABLE statement.
///
/// `rowid` (index 0) is internal to the database and never read.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum PostWorkingCopyColumn {
    Uuid = 1,
    Title = 2,
    Content = 3,
    CreatedAt = 4,
    UpdatedAt = 5,
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
