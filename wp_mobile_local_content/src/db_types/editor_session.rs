//! Editor sessions: one stretch of editing a working copy in an editor, and
//! the requests the editor reported sending during it.

use std::collections::HashMap;

use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use uuid::Uuid;
use wp_api::date::WpGmtDateTime;

use crate::db_types::working_copy::WorkingCopyId;

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
/// ended it in the background. The changes recorded in it are the working
/// copy's versions with its ID, and the requests the editor reported are read
/// separately.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EditorSession {
    pub id: EditorSessionId,
    pub working_copy_id: WorkingCopyId,
    /// Who the app said the editor makes its requests as, recorded as given,
    /// or `None` if it didn't say.
    pub editor_identity: Option<String>,
    pub started_at: WpGmtDateTime,
    /// When a change was last recorded in the working copy, or `None` if none
    /// was.
    pub last_recorded_at: Option<WpGmtDateTime>,
    /// When the session ended normally, or `None` if it hasn't.
    pub ended_at: Option<WpGmtDateTime>,
}

/// A request the editor sent, as it reports it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EditorRequest {
    /// The editor's ID for the request, unique within the session. The
    /// response or failure is reported with it.
    pub request_id: String,
    /// The HTTP method, such as `GET`, `POST` or `PUT`.
    pub method: String,
    /// The REST API path with its query, such as `/wp/v2/posts/42?context=edit`.
    pub path: String,
    /// Header names are stored in lowercase. Credential headers, such as
    /// `authorization` and `cookie`, are never stored.
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

/// The response the editor got to a request, as it reports it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EditorResponse {
    pub status_code: u32,
    /// Header names are stored in lowercase. Credential headers, such as
    /// `set-cookie`, are never stored.
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

/// A request the editor reported during a session, with what came back.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EditorRequestRecord {
    /// The request as stored: header names in lowercase, credential headers
    /// left out.
    pub request: EditorRequest,
    /// When wordpress-rs recorded that the editor sent it.
    pub sent_at: WpGmtDateTime,
    /// `None` if no response or failure was recorded for it.
    pub outcome: Option<EditorRequestOutcome>,
}

/// What came back for a request the editor sent.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum EditorRequestOutcome {
    /// The server answered, with any status.
    Response {
        /// Stored as `EditorRequest::headers` is.
        response: EditorResponse,
        recorded_at: WpGmtDateTime,
    },
    /// The request got no answer, such as when the device was offline.
    Failure {
        reason: String,
        recorded_at: WpGmtDateTime,
    },
}
