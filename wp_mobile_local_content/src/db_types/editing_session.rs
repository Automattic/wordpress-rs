//! Editing sessions: one stretch of work on a working copy, and the requests
//! sent during it.

use std::collections::HashMap;

use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use uuid::Uuid;
use wp_api::date::WpGmtDateTime;

use crate::db_types::working_copy::WorkingCopyId;

uniffi::custom_newtype!(EditingSessionId, String);

/// The ID of an editing session, assigned when it's created: a lowercase,
/// hyphenated v4 UUID.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EditingSessionId(pub String);

impl EditingSessionId {
    pub(crate) fn generate() -> Self {
        Self(Uuid::new_v4().hyphenated().to_string())
    }
}

impl ToSql for EditingSessionId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.as_str()))
    }
}

impl FromSql for EditingSessionId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        String::column_result(value).map(EditingSessionId)
    }
}

/// Where a session's work happens, or who sent a request in it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ActivitySource {
    /// An editor, such as GutenbergKit, named by `name` if given.
    Editor { name: Option<String> },
    /// The app itself, such as one of its screens, or wordpress-rs working
    /// for it, named by `name` if given.
    App { name: Option<String> },
}

/// How a session ended.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SessionEnd {
    /// The session was ended, at `at`.
    Normal { at: WpGmtDateTime },
    /// A later run of the session's client found it not ended, at
    /// `noticed_at`. The work stopped earlier, at the session's
    /// `last_recorded_at` at the latest, or when the session started if
    /// nothing was recorded.
    Abandoned { noticed_at: WpGmtDateTime },
}

/// One stretch of work on a working copy, such as editing it in an editor, or
/// one save from a screen of the app.
///
/// It records only what is known: where the work happened, who opened it,
/// when it started, when a change was last recorded, and how it ended. The
/// changes are the working copy's versions with its ID, and the requests sent
/// in it are read separately.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EditingSession {
    pub id: EditingSessionId,
    pub working_copy_id: WorkingCopyId,
    pub source: ActivitySource,
    /// The client that opened the session, as it names itself.
    pub client: String,
    /// Who the session makes its requests as, recorded as given, or `None` if
    /// it wasn't given.
    pub identity: Option<String>,
    pub started_at: WpGmtDateTime,
    /// When a change was last recorded in the working copy, or `None` if none
    /// was.
    pub last_recorded_at: Option<WpGmtDateTime>,
    /// How the session ended, or `None` if it hasn't.
    pub end: Option<SessionEnd>,
}

/// A request sent in a session.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SessionRequest {
    /// The request's ID, unique within the session. Its response or failure
    /// is recorded with it.
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

/// The response to a request sent in a session.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SessionResponse {
    pub status_code: u32,
    /// Header names are stored in lowercase. Credential headers, such as
    /// `set-cookie`, are never stored.
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

/// A request sent in a session, with what came back.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SessionRequestRecord {
    /// Who sent it.
    pub source: ActivitySource,
    /// The request as stored: header names in lowercase, credential headers
    /// left out.
    pub request: SessionRequest,
    /// When wordpress-rs recorded that it was sent.
    pub sent_at: WpGmtDateTime,
    /// `None` if no response or failure was recorded for it.
    pub outcome: Option<SessionRequestOutcome>,
}

/// What came back for a request the editor sent.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SessionRequestOutcome {
    /// The server answered, with any status.
    Response {
        /// Stored as `SessionRequest::headers` is.
        response: SessionResponse,
        recorded_at: WpGmtDateTime,
    },
    /// The request got no answer, such as when the device was offline.
    Failure {
        reason: String,
        recorded_at: WpGmtDateTime,
    },
}

impl ActivitySource {
    /// The `source` and `source_name` column values.
    pub(crate) fn to_columns(&self) -> (&'static str, Option<&str>) {
        match self {
            Self::Editor { name } => ("editor", name.as_deref()),
            Self::App { name } => ("app", name.as_deref()),
        }
    }

    pub(crate) fn from_columns(source: &str, name: Option<String>) -> Option<Self> {
        match source {
            "editor" => Some(Self::Editor { name }),
            "app" => Some(Self::App { name }),
            _ => None,
        }
    }
}

/// The run of this process: a v4 UUID, the same for every session opened by
/// the process, and different in the next one.
pub(crate) fn current_run() -> &'static str {
    static RUN: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    RUN.get_or_init(|| Uuid::new_v4().hyphenated().to_string())
}
