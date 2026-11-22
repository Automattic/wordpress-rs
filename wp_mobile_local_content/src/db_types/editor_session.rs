//! Editor sessions: one stretch of editing a working copy in an editor.

use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use uuid::Uuid;
use wp_api::date::WpGmtDateTime;

use crate::db_types::{
    working_copy::WorkingCopyId, working_copy_remote_target::RemotePostReference,
};

uniffi::custom_newtype!(EditorSessionId, String);

/// The ID of an editor session, assigned when it's created: a lowercase,
/// hyphenated v4 UUID.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EditorSessionId(pub String);

impl EditorSessionId {
    pub(crate) fn generate() -> Self {
        Self(Uuid::new_v4().hyphenated().to_string())
    }
}

impl ToSql for EditorSessionId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.as_str()))
    }
}

impl FromSql for EditorSessionId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        String::column_result(value).map(EditorSessionId)
    }
}

/// One stretch of editing a working copy in an editor, from opening the editor
/// to closing it.
///
/// It records only what is known: when it started, when a change was last
/// recorded, and when it ended. A session without `ended_at` that no editor
/// has open didn't end normally, such as when the app crashed or the system
/// ended it in the background.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EditorSession {
    pub id: EditorSessionId,
    pub working_copy_id: WorkingCopyId,
    pub started_at: WpGmtDateTime,
    /// When a change was last recorded in the working copy, or `None` if none
    /// was.
    pub last_recorded_at: Option<WpGmtDateTime>,
    /// When the session ended normally, or `None` if it hasn't.
    pub ended_at: Option<WpGmtDateTime>,
    /// The push sent during the session that has no answer yet.
    pub unconfirmed_push: Option<UnconfirmedPush>,
    /// The latest time the editor's version of the post differed from the
    /// server's.
    pub base_mismatch: Option<BaseMismatch>,
}

/// The editor's version of the post, the one it takes as saved, differed
/// from the server's version wordpress-rs has, though both had the same
/// modified date. Only something other than the server changes the editor's
/// version that way.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct BaseMismatch {
    pub detected_at: WpGmtDateTime,
    /// The REST API keys of the fields that differed, such as `excerpt`.
    pub fields: Vec<String>,
}

/// A push that was sent and has no answer yet, so it may or may not have
/// reached the server.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct UnconfirmedPush {
    /// The post the push updates, with the base version its fields started
    /// from. `None` if the push creates the post.
    pub remote_post: Option<RemotePostReference>,
    pub sent_at: WpGmtDateTime,
    /// The request body the push sent, a JSON object.
    pub body: String,
}
