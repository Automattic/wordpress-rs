use std::collections::HashMap;

use rusqlite::{OptionalExtension, Row, types::Type};

use crate::{
    LocalContentError,
    db_types::{
        editor_session::{
            EditorRequest, EditorRequestOutcome, EditorRequestRecord, EditorResponse,
            EditorSession, EditorSessionId,
        },
        working_copy::{WorkingCopyId, date_from_timestamp},
    },
    repository::QueryExecutor,
};

const EDITOR_SESSIONS: &str = "editor_sessions";
const REQUESTS: &str = "editor_session_requests";
const WORKING_COPIES: &str = "working_copies";

/// The current time as a Unix timestamp in seconds, in SQL.
const NOW: &str = "CAST(strftime('%s', 'now') AS INTEGER)";

/// Headers that carry credentials. They're left out of every stored request
/// and response, so the database never holds a credential.
const CREDENTIAL_HEADERS: [&str; 5] = [
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
    "x-wp-nonce",
];

/// Repository for editor sessions and the requests the editor reported in
/// them.
///
/// A session belongs to a working copy and is deleted with it. Its requests
/// are deleted with it.
pub struct EditorSessionRepository;

impl EditorSessionRepository {
    /// Insert a session for `working_copy_id`, started now, and return its newly
    /// generated ID.
    ///
    /// `editor_identity` is stored as given.
    ///
    /// Fails with `WorkingCopyNotFound` if no working copy has the ID.
    pub fn insert(
        &self,
        executor: &impl QueryExecutor,
        working_copy_id: &WorkingCopyId,
        editor_identity: Option<&str>,
    ) -> Result<EditorSessionId, LocalContentError> {
        let id = EditorSessionId::generate();
        let inserted = executor.execute(
            &format!(
                "INSERT INTO {EDITOR_SESSIONS} (uuid, working_copy_rowid, editor_identity, started_at)
                 SELECT ?, rowid, ?, {NOW} FROM {WORKING_COPIES} WHERE uuid = ?"
            ),
            rusqlite::params![id, editor_identity, working_copy_id],
        )?;
        if inserted == 0 {
            return Err(LocalContentError::WorkingCopyNotFound {
                id: working_copy_id.clone(),
            });
        }
        Ok(id)
    }

    /// Set the session's `last_recorded_at` to now.
    ///
    /// Returns `false` if no session has the ID.
    pub fn record_change(
        &self,
        executor: &impl QueryExecutor,
        id: &EditorSessionId,
    ) -> Result<bool, LocalContentError> {
        let updated = executor.execute(
            &format!("UPDATE {EDITOR_SESSIONS} SET last_recorded_at = {NOW} WHERE uuid = ?"),
            [id],
        )?;
        Ok(updated > 0)
    }

    /// Set the session's `ended_at` to now, unless it has already ended.
    ///
    /// Returns `false` if no session has the ID.
    pub fn end(
        &self,
        executor: &impl QueryExecutor,
        id: &EditorSessionId,
    ) -> Result<bool, LocalContentError> {
        let updated = executor.execute(
            &format!(
                "UPDATE {EDITOR_SESSIONS} SET ended_at = COALESCE(ended_at, {NOW}) WHERE uuid = ?"
            ),
            [id],
        )?;
        Ok(updated > 0)
    }

    /// Record that the editor sent `request` during the session, now.
    ///
    /// Header names are stored in lowercase, and credential headers are left
    /// out. Fails with `EditorSessionNotFound` if no session has the ID, and
    /// with `EditorRequestAlreadyRecorded` if the session has a request with
    /// the same `request_id`.
    pub fn record_request_sent(
        &self,
        executor: &impl QueryExecutor,
        id: &EditorSessionId,
        request: &EditorRequest,
    ) -> Result<(), LocalContentError> {
        let session_rowid = select_rowid(executor, id)?
            .ok_or_else(|| LocalContentError::EditorSessionNotFound { id: id.clone() })?;
        let inserted = executor.execute(
            &format!(
                "INSERT INTO {REQUESTS}
                     (editor_session_rowid, request_id, method, path, request_headers, request_body, sent_at)
                 VALUES (?, ?, ?, ?, ?, ?, {NOW})
                 ON CONFLICT (editor_session_rowid, request_id) DO NOTHING"
            ),
            rusqlite::params![
                session_rowid,
                request.request_id,
                request.method,
                request.path,
                headers_to_json(&request.headers),
                request.body,
            ],
        )?;
        if inserted == 0 {
            return Err(LocalContentError::EditorRequestAlreadyRecorded {
                request_id: request.request_id.clone(),
            });
        }
        Ok(())
    }

    /// Record the response to a request the session recorded as sent, now.
    ///
    /// Header names are stored in lowercase, and credential headers are left
    /// out. Fails with `EditorSessionNotFound` if no session has the ID,
    /// `EditorRequestNotFound` if the session has no request with
    /// `request_id`, and `EditorRequestAlreadyCompleted` if the request
    /// already has a response or a failure.
    pub fn record_request_response(
        &self,
        executor: &impl QueryExecutor,
        id: &EditorSessionId,
        request_id: &str,
        response: &EditorResponse,
    ) -> Result<(), LocalContentError> {
        let request_rowid = select_open_request_rowid(executor, id, request_id)?;
        executor.execute(
            &format!(
                "UPDATE {REQUESTS}
                 SET status_code = ?, response_headers = ?, response_body = ?, completed_at = {NOW}
                 WHERE rowid = ?"
            ),
            rusqlite::params![
                response.status_code,
                headers_to_json(&response.headers),
                response.body,
                request_rowid,
            ],
        )?;
        Ok(())
    }

    /// Record that a request the session recorded as sent got no answer, now.
    ///
    /// Fails as `record_request_response` does.
    pub fn record_request_failure(
        &self,
        executor: &impl QueryExecutor,
        id: &EditorSessionId,
        request_id: &str,
        reason: &str,
    ) -> Result<(), LocalContentError> {
        let request_rowid = select_open_request_rowid(executor, id, request_id)?;
        executor.execute(
            &format!("UPDATE {REQUESTS} SET failure = ?, completed_at = {NOW} WHERE rowid = ?"),
            rusqlite::params![reason, request_rowid],
        )?;
        Ok(())
    }

    /// Select a request the session recorded, or `None` if it has none with
    /// `request_id`.
    pub fn select_request(
        &self,
        executor: &impl QueryExecutor,
        id: &EditorSessionId,
        request_id: &str,
    ) -> Result<Option<EditorRequestRecord>, LocalContentError> {
        let mut stmt = executor.prepare(&format!(
            "{SELECT_REQUESTS} WHERE s.uuid = ? AND r.request_id = ?"
        ))?;
        Ok(stmt
            .query_row(rusqlite::params![id, request_id], request_from_row)
            .optional()?)
    }

    /// Select every request the session recorded, in the order the editor
    /// reported sending them.
    pub fn select_requests(
        &self,
        executor: &impl QueryExecutor,
        id: &EditorSessionId,
    ) -> Result<Vec<EditorRequestRecord>, LocalContentError> {
        let mut stmt = executor.prepare(&format!(
            "{SELECT_REQUESTS} WHERE s.uuid = ? ORDER BY r.rowid"
        ))?;
        let requests = stmt
            .query_map([id], request_from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(requests)
    }

    /// Select a session by its ID, or `None` if no session has it.
    pub fn select(
        &self,
        executor: &impl QueryExecutor,
        id: &EditorSessionId,
    ) -> Result<Option<EditorSession>, LocalContentError> {
        let mut stmt = executor.prepare(&format!("{SELECT_SESSIONS} WHERE s.uuid = ?"))?;
        Ok(stmt.query_row([id], session_from_row).optional()?)
    }

    /// Select every session that hasn't ended, oldest first.
    pub fn select_unfinished(
        &self,
        executor: &impl QueryExecutor,
    ) -> Result<Vec<EditorSession>, LocalContentError> {
        let mut stmt = executor.prepare(&format!(
            "{SELECT_SESSIONS} WHERE s.ended_at IS NULL ORDER BY s.started_at, s.rowid"
        ))?;
        let sessions = stmt
            .query_map([], session_from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(sessions)
    }
}

// Internal helpers

/// The columns `session_from_row` reads, in its order.
const SELECT_SESSIONS: &str = "
    SELECT s.uuid, w.uuid, s.editor_identity, s.started_at, s.last_recorded_at, s.ended_at
    FROM editor_sessions s
    JOIN working_copies w ON w.rowid = s.working_copy_rowid";

/// The columns `request_from_row` reads, in its order.
const SELECT_REQUESTS: &str = "
    SELECT r.request_id, r.method, r.path, r.request_headers, r.request_body, r.sent_at,
           r.status_code, r.response_headers, r.response_body, r.failure, r.completed_at
    FROM editor_session_requests r
    JOIN editor_sessions s ON s.rowid = r.editor_session_rowid";

fn date_at(row: &Row, index: usize) -> rusqlite::Result<Option<wp_api::date::WpGmtDateTime>> {
    row.get::<_, Option<i64>>(index)?
        .map(|seconds| {
            date_from_timestamp(seconds).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(index, Type::Integer, Box::new(e))
            })
        })
        .transpose()
}

fn required_date_at(
    row: &Row,
    index: usize,
    name: &str,
) -> rusqlite::Result<wp_api::date::WpGmtDateTime> {
    date_at(row, index)?.ok_or(rusqlite::Error::InvalidColumnType(
        index,
        name.to_string(),
        Type::Null,
    ))
}

fn session_from_row(row: &Row) -> rusqlite::Result<EditorSession> {
    Ok(EditorSession {
        id: row.get(0)?,
        working_copy_id: row.get(1)?,
        editor_identity: row.get(2)?,
        started_at: required_date_at(row, 3, "started_at")?,
        last_recorded_at: date_at(row, 4)?,
        ended_at: date_at(row, 5)?,
    })
}

fn request_from_row(row: &Row) -> rusqlite::Result<EditorRequestRecord> {
    let headers = |index: usize| -> rusqlite::Result<Option<HashMap<String, String>>> {
        row.get::<_, Option<String>>(index)?
            .map(|json| {
                serde_json::from_str(&json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(e))
                })
            })
            .transpose()
    };
    let status_code: Option<u32> = row.get(6)?;
    let failure: Option<String> = row.get(9)?;
    let outcome = match (status_code, failure) {
        (Some(status_code), _) => Some(EditorRequestOutcome::Response {
            response: EditorResponse {
                status_code,
                headers: headers(7)?.unwrap_or_default(),
                body: row.get::<_, Option<Vec<u8>>>(8)?.unwrap_or_default(),
            },
            recorded_at: required_date_at(row, 10, "completed_at")?,
        }),
        (None, Some(reason)) => Some(EditorRequestOutcome::Failure {
            reason,
            recorded_at: required_date_at(row, 10, "completed_at")?,
        }),
        (None, None) => None,
    };
    Ok(EditorRequestRecord {
        request: EditorRequest {
            request_id: row.get(0)?,
            method: row.get(1)?,
            path: row.get(2)?,
            headers: headers(3)?.unwrap_or_default(),
            body: row.get(4)?,
        },
        sent_at: required_date_at(row, 5, "sent_at")?,
        outcome,
    })
}

/// `headers` as a JSON object, with lowercase names and without credential
/// headers. Values of names that differ only in case are joined with `, `.
fn headers_to_json(headers: &HashMap<String, String>) -> String {
    let stored = headers
        .iter()
        .map(|(name, value)| (name.to_ascii_lowercase(), value))
        .filter(|(name, _)| !CREDENTIAL_HEADERS.contains(&name.as_str()))
        .fold(
            std::collections::BTreeMap::<String, String>::new(),
            |mut stored, (name, value)| {
                stored
                    .entry(name)
                    .and_modify(|joined| {
                        joined.push_str(", ");
                        joined.push_str(value);
                    })
                    .or_insert_with(|| value.clone());
                stored
            },
        );
    serde_json::to_string(&stored).expect("A map of strings always serializes")
}

fn select_rowid(
    executor: &impl QueryExecutor,
    id: &EditorSessionId,
) -> Result<Option<i64>, LocalContentError> {
    let mut stmt = executor.prepare(&format!(
        "SELECT rowid FROM {EDITOR_SESSIONS} WHERE uuid = ?"
    ))?;
    Ok(stmt.query_row([id], |row| row.get(0)).optional()?)
}

/// The rowid of the session's request `request_id`, which must have neither a
/// response nor a failure yet.
fn select_open_request_rowid(
    executor: &impl QueryExecutor,
    id: &EditorSessionId,
    request_id: &str,
) -> Result<i64, LocalContentError> {
    let session_rowid = select_rowid(executor, id)?
        .ok_or_else(|| LocalContentError::EditorSessionNotFound { id: id.clone() })?;
    let mut stmt = executor.prepare(&format!(
        "SELECT rowid, completed_at IS NOT NULL FROM {REQUESTS}
         WHERE editor_session_rowid = ? AND request_id = ?"
    ))?;
    match stmt
        .query_row(rusqlite::params![session_rowid, request_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, bool>(1)?))
        })
        .optional()?
    {
        Some((rowid, false)) => Ok(rowid),
        Some((_, true)) => Err(LocalContentError::EditorRequestAlreadyCompleted {
            request_id: request_id.to_string(),
        }),
        None => Err(LocalContentError::EditorRequestNotFound {
            request_id: request_id.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        MIGRATIONS,
        db_types::{
            working_copy::{
                FeaturedImage, PostWorkingCopyFields, PublishDate, StatusSetting,
                WorkingCopyCommonFields,
            },
            working_copy_remote_target::{SiteIdentity, WorkingCopyRemoteTarget},
        },
        repository::working_copies::WorkingCopyRepository,
    };
    use rusqlite::Connection;
    use std::sync::Arc;
    use wp_api::{
        WpAdditionalFields,
        parsed_url::ParsedUrl,
        posts::{PostFormat, PostMeta},
    };
    use wp_mobile_sqlite::MigrationManager;

    fn connection_with_working_copy() -> (Connection, WorkingCopyId) {
        let mut conn = Connection::open_in_memory().expect("Failed to open in-memory database");
        MigrationManager::new(&conn, &MIGRATIONS)
            .expect("Failed to create MigrationManager")
            .perform_migrations()
            .expect("Migrations should succeed");
        let fields = PostWorkingCopyFields {
            common: WorkingCopyCommonFields {
                title: "A post".to_string(),
                content: String::new(),
                excerpt: String::new(),
                slug: None,
                status: StatusSetting::Draft { password: None },
                publish_date: PublishDate::Immediately,
                author: None,
                featured_image: FeaturedImage::None,
                comment_status: None,
                ping_status: None,
                template: None,
                menu_order: None,
            },
            categories: vec![],
            tags: vec![],
            format: PostFormat::Standard,
            sticky: false,
            meta: PostMeta::new(),
            additional_fields: WpAdditionalFields::new(),
        };
        let remote_target = WorkingCopyRemoteTarget {
            site: SiteIdentity {
                url: Arc::new(ParsedUrl::parse("https://example.com").expect("URL should parse")),
                wordpress_com_site_id: None,
            },
            remote_post: None,
        };
        let tx = conn.transaction().expect("Failed to begin transaction");
        let id = WorkingCopyRepository
            .insert_post(&tx, &fields, &remote_target)
            .expect("Insert should succeed");
        tx.commit().expect("Commit should succeed");
        (conn, id)
    }

    fn insert_session(conn: &Connection, working_copy_id: &WorkingCopyId) -> EditorSessionId {
        EditorSessionRepository
            .insert(conn, working_copy_id, None)
            .expect("Insert should succeed")
    }

    fn select(conn: &Connection, id: &EditorSessionId) -> EditorSession {
        EditorSessionRepository
            .select(conn, id)
            .expect("Select should succeed")
            .expect("The session should exist")
    }

    fn request(request_id: &str, headers: &[(&str, &str)]) -> EditorRequest {
        EditorRequest {
            request_id: request_id.to_string(),
            method: "PUT".to_string(),
            path: "/wp/v2/posts/42".to_string(),
            headers: headers
                .iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
            body: Some(br#"{"title":"Saved"}"#.to_vec()),
        }
    }

    fn requests(conn: &Connection, id: &EditorSessionId) -> Vec<EditorRequestRecord> {
        EditorSessionRepository
            .select_requests(conn, id)
            .expect("Select should succeed")
    }

    #[test]
    fn test_session_records_what_happened_in_it() {
        let (conn, working_copy_id) = connection_with_working_copy();

        let id = EditorSessionRepository
            .insert(&conn, &working_copy_id, Some("user-7@example.com"))
            .expect("Insert should succeed");
        let started = select(&conn, &id);
        assert_eq!(started.working_copy_id, working_copy_id);
        assert_eq!(
            started.editor_identity.as_deref(),
            Some("user-7@example.com")
        );
        assert_eq!((started.last_recorded_at, started.ended_at), (None, None));

        assert!(
            EditorSessionRepository
                .record_change(&conn, &id)
                .expect("Recording should succeed")
        );
        assert!(
            EditorSessionRepository
                .end(&conn, &id)
                .expect("Ending should succeed")
        );

        let ended = select(&conn, &id);
        assert!(ended.last_recorded_at.is_some());
        assert!(ended.ended_at.is_some());
    }

    #[test]
    fn test_only_sessions_that_have_not_ended_are_unfinished() {
        let (conn, working_copy_id) = connection_with_working_copy();
        let ended = insert_session(&conn, &working_copy_id);
        let unfinished = insert_session(&conn, &working_copy_id);
        EditorSessionRepository
            .end(&conn, &ended)
            .expect("Ending should succeed");

        let ids: Vec<_> = EditorSessionRepository
            .select_unfinished(&conn)
            .expect("Select should succeed")
            .into_iter()
            .map(|session| session.id)
            .collect();

        assert_eq!(ids, vec![unfinished]);
    }

    #[test]
    fn test_requests_are_kept_in_order_with_what_came_back() {
        let (conn, working_copy_id) = connection_with_working_copy();
        let id = insert_session(&conn, &working_copy_id);

        for request_id in ["save", "read", "lost"] {
            EditorSessionRepository
                .record_request_sent(&conn, &id, &request(request_id, &[]))
                .expect("Recording should succeed");
        }
        EditorSessionRepository
            .record_request_failure(&conn, &id, "read", "The device is offline")
            .expect("Recording should succeed");
        let response = EditorResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: br#"{"id":42}"#.to_vec(),
        };
        EditorSessionRepository
            .record_request_response(&conn, &id, "save", &response)
            .expect("Recording should succeed");

        let recorded = requests(&conn, &id);
        let ids: Vec<_> = recorded
            .iter()
            .map(|record| record.request.request_id.as_str())
            .collect();
        assert_eq!(ids, ["save", "read", "lost"]);
        assert!(matches!(
            &recorded[0].outcome,
            Some(EditorRequestOutcome::Response { response: stored, .. }) if *stored == response
        ));
        assert!(matches!(
            &recorded[1].outcome,
            Some(EditorRequestOutcome::Failure { reason, .. }) if reason == "The device is offline"
        ));
        assert_eq!(recorded[2].outcome, None);
        assert_eq!(recorded[2].request.body, request("lost", &[]).body);
    }

    #[test]
    fn test_credential_headers_are_never_stored() {
        let (conn, working_copy_id) = connection_with_working_copy();
        let id = insert_session(&conn, &working_copy_id);

        EditorSessionRepository
            .record_request_sent(
                &conn,
                &id,
                &request(
                    "save",
                    &[
                        ("Authorization", "Bearer secret"),
                        ("Cookie", "wordpress_logged_in=secret"),
                        ("X-WP-Nonce", "secret"),
                        ("Content-Type", "application/json"),
                    ],
                ),
            )
            .expect("Recording should succeed");
        EditorSessionRepository
            .record_request_response(
                &conn,
                &id,
                "save",
                &EditorResponse {
                    status_code: 200,
                    headers: HashMap::from([
                        ("Set-Cookie".to_string(), "secret".to_string()),
                        ("X-WP-Total".to_string(), "1".to_string()),
                    ]),
                    body: vec![],
                },
            )
            .expect("Recording should succeed");

        let stored: String = conn
            .query_row(
                &format!("SELECT request_headers || response_headers FROM {REQUESTS}"),
                [],
                |row| row.get(0),
            )
            .expect("Select should succeed");
        assert!(!stored.contains("secret"), "Stored headers: {stored}");
        assert_eq!(
            requests(&conn, &id)[0].request.headers,
            HashMap::from([("content-type".to_string(), "application/json".to_string())])
        );
    }

    #[test]
    fn test_request_reports_must_follow_the_request() {
        let (conn, working_copy_id) = connection_with_working_copy();
        let id = insert_session(&conn, &working_copy_id);
        let response = EditorResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: vec![],
        };

        assert_eq!(
            EditorSessionRepository.record_request_response(&conn, &id, "unknown", &response),
            Err(LocalContentError::EditorRequestNotFound {
                request_id: "unknown".to_string()
            })
        );

        EditorSessionRepository
            .record_request_sent(&conn, &id, &request("save", &[]))
            .expect("Recording should succeed");
        assert_eq!(
            EditorSessionRepository.record_request_sent(&conn, &id, &request("save", &[])),
            Err(LocalContentError::EditorRequestAlreadyRecorded {
                request_id: "save".to_string()
            })
        );
        EditorSessionRepository
            .record_request_response(&conn, &id, "save", &response)
            .expect("Recording should succeed");
        assert_eq!(
            EditorSessionRepository.record_request_failure(&conn, &id, "save", "Lost"),
            Err(LocalContentError::EditorRequestAlreadyCompleted {
                request_id: "save".to_string()
            })
        );
    }

    #[test]
    fn test_session_needs_an_existing_working_copy_and_goes_with_it() {
        let (conn, working_copy_id) = connection_with_working_copy();
        let missing = WorkingCopyId("not-a-working-copy".to_string());

        assert_eq!(
            EditorSessionRepository.insert(&conn, &missing, None),
            Err(LocalContentError::WorkingCopyNotFound { id: missing })
        );

        let id = insert_session(&conn, &working_copy_id);
        EditorSessionRepository
            .record_request_sent(&conn, &id, &request("save", &[]))
            .expect("Recording should succeed");
        WorkingCopyRepository
            .delete_post(&conn, &working_copy_id)
            .expect("Delete should succeed");

        assert_eq!(
            EditorSessionRepository
                .select(&conn, &id)
                .expect("Select should succeed"),
            None
        );
        let remaining: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {REQUESTS}"), [], |row| {
                row.get(0)
            })
            .expect("Count should succeed");
        assert_eq!(remaining, 0);
    }
}
