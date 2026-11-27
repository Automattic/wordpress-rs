//! The history of a working copy: one version each time its fields were saved
//! with a change.

use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use uuid::Uuid;

use crate::db_types::editing_session::EditingSessionId;

uniffi::custom_newtype!(WorkingCopyVersionId, String);

/// The ID of a working copy's version, assigned when it's saved: a lowercase,
/// hyphenated v4 UUID.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkingCopyVersionId(pub String);

impl WorkingCopyVersionId {
    pub(crate) fn generate() -> Self {
        Self(Uuid::new_v4().hyphenated().to_string())
    }
}

impl ToSql for WorkingCopyVersionId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.as_str()))
    }
}

impl FromSql for WorkingCopyVersionId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        String::column_result(value).map(WorkingCopyVersionId)
    }
}

/// One version in a working copy's history. Its fields are read separately,
/// by its ID.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct WorkingCopyVersion {
    pub id: WorkingCopyVersionId,
    /// The version this one was made from, or `None` for the working copy's
    /// first version.
    pub parent: Option<WorkingCopyVersionId>,
    /// The editing session the version was saved in.
    pub session_id: EditingSessionId,
    /// When the version was saved, as an ISO 8601 UTC timestamp.
    pub created_at: String,
}
