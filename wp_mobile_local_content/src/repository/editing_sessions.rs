use std::collections::HashMap;

use rusqlite::{OptionalExtension, Row, types::Type};
use wp_api::date::WpGmtDateTime;

use crate::{
    LocalContentError,
    db_types::{
        editing_session::{
            ActivitySource, EditingSession, EditingSessionId, SessionEnd, SessionRequest,
            SessionRequestOutcome, SessionRequestRecord, SessionResponse, current_run,
        },
        working_copy::{WorkingCopyId, date_from_timestamp},
    },
    repository::QueryExecutor,
};

const EDITING_SESSIONS: &str = "editing_sessions";
const REQUESTS: &str = "editing_session_requests";
const WORKING_COPIES: &str = "working_copies";

/// The current time as a Unix timestamp in seconds, in SQL.
const NOW: &str = "CAST(strftime('%s', 'now') AS INTEGER)";

/// The `uuid` of the newest version of the working copy `w`, in SQL, or NULL
/// if it has none yet.
const NEWEST_VERSION: &str = "(SELECT v.uuid FROM working_copy_versions v
      WHERE v.working_copy_rowid = w.rowid ORDER BY v.rowid DESC LIMIT 1)";

/// Headers that carry credentials. They're left out of every stored request
/// and response, so the database never holds a credential.
const CREDENTIAL_HEADERS: [&str; 5] = [
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
    "x-wp-nonce",
];

/// Repository for editing sessions and the requests sent in them.
///
/// A session belongs to a working copy and is deleted with it. Its versions
/// and its requests are deleted with the session.
pub struct EditingSessionRepository;

impl EditingSessionRepository {
    /// Insert a session for `working_copy_id`, started now in this process's
    /// run, and return its newly generated ID.
    ///
    /// `client` and `identity` are stored as given.
    ///
    /// Fails with `WorkingCopyNotFound` if no working copy has the ID.
    pub fn insert(
        &self,
        executor: &impl QueryExecutor,
        working_copy_id: &WorkingCopyId,
        source: &ActivitySource,
        client: &str,
        identity: Option<&str>,
    ) -> Result<EditingSessionId, LocalContentError> {
        let id = EditingSessionId::generate();
        let (source, source_name) = source.to_columns();
        let inserted = executor.execute(
            &format!(
                "INSERT INTO {EDITING_SESSIONS}
                     (uuid, working_copy_rowid, source, source_name, client, run,
                      started_from_version, identity, started_at)
                 SELECT ?, w.rowid, ?, ?, ?, ?, {NEWEST_VERSION}, ?, {NOW}
                 FROM {WORKING_COPIES} w WHERE w.uuid = ?"
            ),
            rusqlite::params![
                id,
                source,
                source_name,
                client,
                current_run(),
                identity,
                working_copy_id
            ],
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
        id: &EditingSessionId,
    ) -> Result<bool, LocalContentError> {
        let updated = executor.execute(
            &format!("UPDATE {EDITING_SESSIONS} SET last_recorded_at = {NOW} WHERE uuid = ?"),
            [id],
        )?;
        Ok(updated > 0)
    }

    /// End the session normally, now, unless it has already ended.
    ///
    /// Returns `false` if no session has the ID.
    pub fn end(
        &self,
        executor: &impl QueryExecutor,
        id: &EditingSessionId,
    ) -> Result<bool, LocalContentError> {
        executor.execute(
            &format!(
                "UPDATE {EDITING_SESSIONS} SET ended_at = {NOW}, ending = 'normal'
                 WHERE uuid = ? AND ended_at IS NULL"
            ),
            [id],
        )?;
        Ok(select_rowid(executor, id)?.is_some())
    }

    /// Record every session of `client` that was opened in an earlier run and
    /// hasn't ended as abandoned, now, and return their IDs, oldest first.
    ///
    /// Sessions opened in this process's run, and other clients' sessions,
    /// are left alone.
    pub fn record_abandoned(
        &self,
        executor: &impl QueryExecutor,
        client: &str,
    ) -> Result<Vec<EditingSessionId>, LocalContentError> {
        let mut stmt = executor.prepare(&format!(
            "UPDATE {EDITING_SESSIONS} SET ended_at = {NOW}, ending = 'abandoned'
             WHERE ended_at IS NULL AND client = ? AND run <> ?
             RETURNING rowid, uuid"
        ))?;
        let mut ids = stmt
            .query_map(rusqlite::params![client, current_run()], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, EditingSessionId>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        ids.sort_by_key(|(rowid, _)| *rowid);
        Ok(ids.into_iter().map(|(_, id)| id).collect())
    }

    /// Record that `source` sent `request` during the session, now.
    ///
    /// Header names are stored in lowercase, and credential headers are left
    /// out. Fails with `EditingSessionNotFound` if no session has the ID, and
    /// with `SessionRequestAlreadyRecorded` if the session has a request with
    /// the same `request_id`.
    pub fn record_request_sent(
        &self,
        executor: &impl QueryExecutor,
        id: &EditingSessionId,
        source: &ActivitySource,
        request: &SessionRequest,
    ) -> Result<(), LocalContentError> {
        let session_rowid = select_rowid(executor, id)?
            .ok_or_else(|| LocalContentError::EditingSessionNotFound { id: id.clone() })?;
        let (source, source_name) = source.to_columns();
        let inserted = executor.execute(
            &format!(
                "INSERT INTO {REQUESTS}
                     (session_rowid, request_id, source, source_name, method, path,
                      request_headers, request_body, sent_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, {NOW})
                 ON CONFLICT (session_rowid, request_id) DO NOTHING"
            ),
            rusqlite::params![
                session_rowid,
                request.request_id,
                source,
                source_name,
                request.method,
                request.path,
                headers_to_json(&request.headers),
                request.body,
            ],
        )?;
        if inserted == 0 {
            return Err(LocalContentError::SessionRequestAlreadyRecorded {
                request_id: request.request_id.clone(),
            });
        }
        Ok(())
    }

    /// Record the response to a request the session recorded as sent, now.
    ///
    /// Header names are stored in lowercase, and credential headers are left
    /// out. Fails with `EditingSessionNotFound` if no session has the ID,
    /// `SessionRequestNotFound` if the session has no request with
    /// `request_id`, and `SessionRequestAlreadyCompleted` if the request
    /// already has a response or a failure.
    pub fn record_request_response(
        &self,
        executor: &impl QueryExecutor,
        id: &EditingSessionId,
        request_id: &str,
        response: &SessionResponse,
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
        id: &EditingSessionId,
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
        id: &EditingSessionId,
        request_id: &str,
    ) -> Result<Option<SessionRequestRecord>, LocalContentError> {
        let mut stmt = executor.prepare(&format!(
            "{SELECT_REQUESTS} WHERE s.uuid = ? AND r.request_id = ?"
        ))?;
        Ok(stmt
            .query_row(rusqlite::params![id, request_id], request_from_row)
            .optional()?)
    }

    /// Select every request the session recorded, in the order they were sent.
    pub fn select_requests(
        &self,
        executor: &impl QueryExecutor,
        id: &EditingSessionId,
    ) -> Result<Vec<SessionRequestRecord>, LocalContentError> {
        let mut stmt = executor.prepare(&format!(
            "{SELECT_REQUESTS} WHERE s.uuid = ? ORDER BY r.rowid"
        ))?;
        let requests = stmt
            .query_map([id], request_from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(requests)
    }

    /// Select every session of a working copy, oldest first.
    pub fn select_by_working_copy(
        &self,
        executor: &impl QueryExecutor,
        working_copy_id: &WorkingCopyId,
    ) -> Result<Vec<EditingSession>, LocalContentError> {
        let mut stmt = executor.prepare(&format!(
            "{SELECT_SESSIONS} WHERE w.uuid = ? ORDER BY s.rowid"
        ))?;
        let sessions = stmt
            .query_map([working_copy_id], session_from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(sessions)
    }

    /// Select a session by its ID, or `None` if no session has it.
    pub fn select(
        &self,
        executor: &impl QueryExecutor,
        id: &EditingSessionId,
    ) -> Result<Option<EditingSession>, LocalContentError> {
        let mut stmt = executor.prepare(&format!("{SELECT_SESSIONS} WHERE s.uuid = ?"))?;
        Ok(stmt.query_row([id], session_from_row).optional()?)
    }
}

// Internal helpers

/// Insert a session for the working copy at `working_copy_rowid` that starts
/// and ends now, for one change made outside any other session, and return
/// its rowid.
pub(crate) fn insert_one_off(
    executor: &impl QueryExecutor,
    working_copy_rowid: i64,
    client: &str,
) -> Result<i64, LocalContentError> {
    let id = EditingSessionId::generate();
    let mut stmt = executor.prepare(&format!(
        "INSERT INTO {EDITING_SESSIONS}
             (uuid, working_copy_rowid, source, client, run, started_from_version, started_at,
              last_recorded_at, ended_at, ending)
         SELECT ?, w.rowid, 'app', ?, ?, {NEWEST_VERSION}, {NOW}, {NOW}, {NOW}, 'normal'
         FROM {WORKING_COPIES} w WHERE w.rowid = ?
         RETURNING rowid"
    ))?;
    Ok(stmt.query_row(
        rusqlite::params![id, client, current_run(), working_copy_rowid],
        |row| row.get(0),
    )?)
}

pub(crate) fn select_rowid(
    executor: &impl QueryExecutor,
    id: &EditingSessionId,
) -> Result<Option<i64>, LocalContentError> {
    let mut stmt = executor.prepare(&format!(
        "SELECT rowid FROM {EDITING_SESSIONS} WHERE uuid = ?"
    ))?;
    Ok(stmt.query_row([id], |row| row.get(0)).optional()?)
}

/// The columns `session_from_row` reads, in its order.
const SELECT_SESSIONS: &str = "
    SELECT s.uuid, w.uuid, s.source, s.source_name, s.client, s.identity, s.started_at,
           s.last_recorded_at, s.ended_at, s.ending, s.started_from_version
    FROM editing_sessions s
    JOIN working_copies w ON w.rowid = s.working_copy_rowid";

/// The columns `request_from_row` reads, in its order.
const SELECT_REQUESTS: &str = "
    SELECT r.request_id, r.method, r.path, r.request_headers, r.request_body, r.sent_at,
           r.status_code, r.response_headers, r.response_body, r.failure, r.completed_at,
           r.source, r.source_name
    FROM editing_session_requests r
    JOIN editing_sessions s ON s.rowid = r.session_rowid";

fn date_at(row: &Row, index: usize) -> rusqlite::Result<Option<WpGmtDateTime>> {
    row.get::<_, Option<i64>>(index)?
        .map(|seconds| {
            date_from_timestamp(seconds).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(index, Type::Integer, Box::new(e))
            })
        })
        .transpose()
}

fn required_date_at(row: &Row, index: usize, name: &str) -> rusqlite::Result<WpGmtDateTime> {
    date_at(row, index)?.ok_or(rusqlite::Error::InvalidColumnType(
        index,
        name.to_string(),
        Type::Null,
    ))
}

fn source_at(row: &Row, index: usize) -> rusqlite::Result<ActivitySource> {
    let source: String = row.get(index)?;
    ActivitySource::from_columns(&source, row.get(index + 1)?).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            Type::Text,
            format!("Unknown source {source}").into(),
        )
    })
}

fn session_from_row(row: &Row) -> rusqlite::Result<EditingSession> {
    let end = match (
        date_at(row, 8)?,
        row.get::<_, Option<String>>(9)?.as_deref(),
    ) {
        (Some(at), Some("normal")) => Some(SessionEnd::Normal { at }),
        (Some(noticed_at), Some("abandoned")) => Some(SessionEnd::Abandoned { noticed_at }),
        _ => None,
    };
    Ok(EditingSession {
        id: row.get(0)?,
        working_copy_id: row.get(1)?,
        source: source_at(row, 2)?,
        client: row.get(4)?,
        identity: row.get(5)?,
        started_from: row.get(10)?,
        started_at: required_date_at(row, 6, "started_at")?,
        last_recorded_at: date_at(row, 7)?,
        end,
    })
}

fn request_from_row(row: &Row) -> rusqlite::Result<SessionRequestRecord> {
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
        (Some(status_code), _) => Some(SessionRequestOutcome::Response {
            response: SessionResponse {
                status_code,
                headers: headers(7)?.unwrap_or_default(),
                body: row.get::<_, Option<Vec<u8>>>(8)?.unwrap_or_default(),
            },
            recorded_at: required_date_at(row, 10, "completed_at")?,
        }),
        (None, Some(reason)) => Some(SessionRequestOutcome::Failure {
            reason,
            recorded_at: required_date_at(row, 10, "completed_at")?,
        }),
        (None, None) => None,
    };
    Ok(SessionRequestRecord {
        source: source_at(row, 11)?,
        request: SessionRequest {
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

/// The rowid of the session's request `request_id`, which must have neither a
/// response nor a failure yet.
fn select_open_request_rowid(
    executor: &impl QueryExecutor,
    id: &EditingSessionId,
    request_id: &str,
) -> Result<i64, LocalContentError> {
    let session_rowid = select_rowid(executor, id)?
        .ok_or_else(|| LocalContentError::EditingSessionNotFound { id: id.clone() })?;
    let mut stmt = executor.prepare(&format!(
        "SELECT rowid, completed_at IS NOT NULL FROM {REQUESTS}
         WHERE session_rowid = ? AND request_id = ?"
    ))?;
    match stmt
        .query_row(rusqlite::params![session_rowid, request_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, bool>(1)?))
        })
        .optional()?
    {
        Some((rowid, false)) => Ok(rowid),
        Some((_, true)) => Err(LocalContentError::SessionRequestAlreadyCompleted {
            request_id: request_id.to_string(),
        }),
        None => Err(LocalContentError::SessionRequestNotFound {
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

    const CLIENT: &str = "test-app";

    fn editor() -> ActivitySource {
        ActivitySource::Editor {
            name: Some("GutenbergKit".to_string()),
        }
    }

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
            .insert_post(&tx, &fields, &remote_target, CLIENT)
            .expect("Insert should succeed");
        tx.commit().expect("Commit should succeed");
        (conn, id)
    }

    fn insert_session(conn: &Connection, working_copy_id: &WorkingCopyId) -> EditingSessionId {
        EditingSessionRepository
            .insert(conn, working_copy_id, &editor(), CLIENT, None)
            .expect("Insert should succeed")
    }

    fn select(conn: &Connection, id: &EditingSessionId) -> EditingSession {
        EditingSessionRepository
            .select(conn, id)
            .expect("Select should succeed")
            .expect("The session should exist")
    }

    fn request(request_id: &str, headers: &[(&str, &str)]) -> SessionRequest {
        SessionRequest {
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

    fn record_sent(conn: &Connection, id: &EditingSessionId, request: &SessionRequest) {
        EditingSessionRepository
            .record_request_sent(conn, id, &editor(), request)
            .expect("Recording should succeed");
    }

    fn requests(conn: &Connection, id: &EditingSessionId) -> Vec<SessionRequestRecord> {
        EditingSessionRepository
            .select_requests(conn, id)
            .expect("Select should succeed")
    }

    #[test]
    fn test_session_records_what_happened_in_it() {
        let (conn, working_copy_id) = connection_with_working_copy();

        let id = EditingSessionRepository
            .insert(
                &conn,
                &working_copy_id,
                &editor(),
                CLIENT,
                Some("identity-hash"),
            )
            .expect("Insert should succeed");
        let started = select(&conn, &id);
        assert_eq!(started.working_copy_id, working_copy_id);
        assert_eq!(started.source, editor());
        assert_eq!(started.client, CLIENT);
        assert_eq!(started.identity.as_deref(), Some("identity-hash"));
        let newest = WorkingCopyRepository
            .select_versions(&conn, &working_copy_id)
            .expect("Select should succeed")
            .expect("The working copy should exist")
            .pop()
            .expect("The working copy has a version");
        assert_eq!(started.started_from, Some(newest.id.clone()));
        let creating = EditingSessionRepository
            .select(&conn, &newest.session_id)
            .expect("Select should succeed")
            .expect("The creating session should exist");
        assert_eq!(
            creating.started_from, None,
            "The session that created the working copy started from nothing"
        );
        assert_eq!((started.last_recorded_at, started.end), (None, None));

        assert!(
            EditingSessionRepository
                .record_change(&conn, &id)
                .expect("Recording should succeed")
        );
        assert!(
            EditingSessionRepository
                .end(&conn, &id)
                .expect("Ending should succeed")
        );

        let ended = select(&conn, &id);
        assert!(ended.last_recorded_at.is_some());
        assert!(matches!(ended.end, Some(SessionEnd::Normal { .. })));
    }

    #[test]
    fn test_only_the_client_sessions_from_earlier_runs_are_recorded_as_abandoned() {
        let (conn, working_copy_id) = connection_with_working_copy();
        let earlier_run = insert_session(&conn, &working_copy_id);
        let this_run = insert_session(&conn, &working_copy_id);
        let ended = insert_session(&conn, &working_copy_id);
        let another_client = EditingSessionRepository
            .insert(&conn, &working_copy_id, &editor(), "share-extension", None)
            .expect("Insert should succeed");
        EditingSessionRepository
            .end(&conn, &ended)
            .expect("Ending should succeed");
        conn.execute(
            &format!("UPDATE {EDITING_SESSIONS} SET run = 'an-earlier-run' WHERE uuid IN (?, ?)"),
            rusqlite::params![earlier_run, another_client],
        )
        .expect("Update should succeed");

        let abandoned = EditingSessionRepository
            .record_abandoned(&conn, CLIENT)
            .expect("Recording should succeed");

        assert_eq!(abandoned, std::slice::from_ref(&earlier_run));
        assert!(matches!(
            select(&conn, &earlier_run).end,
            Some(SessionEnd::Abandoned { .. })
        ));
        assert_eq!(select(&conn, &this_run).end, None);
        assert_eq!(select(&conn, &another_client).end, None);
        assert!(matches!(
            select(&conn, &ended).end,
            Some(SessionEnd::Normal { .. })
        ));
    }

    #[test]
    fn test_requests_are_kept_in_order_with_their_source_and_what_came_back() {
        let (conn, working_copy_id) = connection_with_working_copy();
        let id = insert_session(&conn, &working_copy_id);
        let app = ActivitySource::App {
            name: Some("post-settings".to_string()),
        };

        record_sent(&conn, &id, &request("save", &[]));
        EditingSessionRepository
            .record_request_sent(&conn, &id, &app, &request("read", &[]))
            .expect("Recording should succeed");
        record_sent(&conn, &id, &request("lost", &[]));
        EditingSessionRepository
            .record_request_failure(&conn, &id, "read", "The device is offline")
            .expect("Recording should succeed");
        let response = SessionResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: br#"{"id":42}"#.to_vec(),
        };
        EditingSessionRepository
            .record_request_response(&conn, &id, "save", &response)
            .expect("Recording should succeed");

        let recorded = requests(&conn, &id);
        let ids: Vec<_> = recorded
            .iter()
            .map(|record| record.request.request_id.as_str())
            .collect();
        assert_eq!(ids, ["save", "read", "lost"]);
        assert_eq!(recorded[1].source, app);
        assert!(matches!(
            &recorded[0].outcome,
            Some(SessionRequestOutcome::Response { response: stored, .. }) if *stored == response
        ));
        assert!(matches!(
            &recorded[1].outcome,
            Some(SessionRequestOutcome::Failure { reason, .. }) if reason == "The device is offline"
        ));
        assert_eq!(recorded[2].outcome, None);
    }

    #[test]
    fn test_credential_headers_are_never_stored() {
        let (conn, working_copy_id) = connection_with_working_copy();
        let id = insert_session(&conn, &working_copy_id);

        record_sent(
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
        );
        EditingSessionRepository
            .record_request_response(
                &conn,
                &id,
                "save",
                &SessionResponse {
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
        let response = SessionResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: vec![],
        };

        assert_eq!(
            EditingSessionRepository.record_request_response(&conn, &id, "unknown", &response),
            Err(LocalContentError::SessionRequestNotFound {
                request_id: "unknown".to_string()
            })
        );

        record_sent(&conn, &id, &request("save", &[]));
        assert_eq!(
            EditingSessionRepository.record_request_sent(
                &conn,
                &id,
                &editor(),
                &request("save", &[])
            ),
            Err(LocalContentError::SessionRequestAlreadyRecorded {
                request_id: "save".to_string()
            })
        );
        EditingSessionRepository
            .record_request_response(&conn, &id, "save", &response)
            .expect("Recording should succeed");
        assert_eq!(
            EditingSessionRepository.record_request_failure(&conn, &id, "save", "Lost"),
            Err(LocalContentError::SessionRequestAlreadyCompleted {
                request_id: "save".to_string()
            })
        );
    }

    /// A session is the unit the history is kept in: removing one removes its
    /// versions and its requests.
    #[test]
    fn test_a_session_takes_its_versions_and_requests_with_it() {
        let (mut conn, working_copy_id) = connection_with_working_copy();
        let id = insert_session(&conn, &working_copy_id);
        record_sent(&conn, &id, &request("save", &[]));
        let tx = conn.transaction().expect("Failed to begin transaction");
        let mut fields = WorkingCopyRepository
            .select_post(&tx, &working_copy_id)
            .expect("Select should succeed")
            .expect("The working copy should exist")
            .fields;
        fields.common.title = "In the session".to_string();
        WorkingCopyRepository
            .update_post(
                &tx,
                &working_copy_id,
                &fields,
                crate::repository::working_copies::VersionSession::Existing(&id),
            )
            .expect("Update should succeed");
        tx.commit().expect("Commit should succeed");
        let count = |table: &str| -> i64 {
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("Count should succeed")
        };
        assert_eq!(count("working_copy_versions"), 2);

        conn.execute(
            &format!("DELETE FROM {EDITING_SESSIONS} WHERE uuid = ?"),
            [&id],
        )
        .expect("Delete should succeed");

        assert_eq!(count("working_copy_versions"), 1);
        assert_eq!(count(REQUESTS), 0);
    }

    #[test]
    fn test_session_needs_an_existing_working_copy_and_goes_with_it() {
        let (conn, working_copy_id) = connection_with_working_copy();
        let missing = WorkingCopyId("not-a-working-copy".to_string());

        assert_eq!(
            EditingSessionRepository.insert(&conn, &missing, &editor(), CLIENT, None),
            Err(LocalContentError::WorkingCopyNotFound { id: missing })
        );

        let id = insert_session(&conn, &working_copy_id);
        WorkingCopyRepository
            .delete_post(&conn, &working_copy_id)
            .expect("Delete should succeed");

        assert_eq!(
            EditingSessionRepository
                .select(&conn, &id)
                .expect("Select should succeed"),
            None
        );
    }
}
