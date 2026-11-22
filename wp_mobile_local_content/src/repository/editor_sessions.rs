use rusqlite::{OptionalExtension, Row, types::Type};

use crate::{
    LocalContentError,
    db_types::{
        editor_session::{EditorSession, EditorSessionId, UnconfirmedPush},
        working_copy::{WorkingCopyId, date_from_timestamp},
        working_copy_remote_target::RemotePostReference,
    },
    repository::QueryExecutor,
};
use wp_api::posts::PostId;

const EDITOR_SESSIONS: &str = "editor_sessions";
const UNCONFIRMED_PUSHES: &str = "editor_session_unconfirmed_pushes";
const WORKING_COPIES: &str = "working_copies";

/// The current time as a Unix timestamp in seconds, in SQL.
const NOW: &str = "CAST(strftime('%s', 'now') AS INTEGER)";

/// Repository for editor sessions and their unconfirmed pushes.
///
/// A session belongs to a working copy and is deleted with it.
pub struct EditorSessionRepository;

impl EditorSessionRepository {
    /// Insert a session for `working_copy_id`, started now, and return its newly
    /// generated ID.
    ///
    /// Fails with `WorkingCopyNotFound` if no working copy has the ID.
    pub fn insert(
        &self,
        executor: &impl QueryExecutor,
        working_copy_id: &WorkingCopyId,
    ) -> Result<EditorSessionId, LocalContentError> {
        let id = EditorSessionId::generate();
        let inserted = executor.execute(
            &format!(
                "INSERT INTO {EDITOR_SESSIONS} (uuid, working_copy_rowid, started_at)
                 SELECT ?, rowid, {NOW} FROM {WORKING_COPIES} WHERE uuid = ?"
            ),
            rusqlite::params![id, working_copy_id],
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

    /// Record a push sent now that has no answer yet, replacing the one the
    /// session had.
    ///
    /// `remote_post` is the post the push updates, with its base version, or
    /// `None` if the push creates the post. `body` must be a JSON object.
    /// Returns `false` if no session has the ID.
    pub fn record_unconfirmed_push(
        &self,
        executor: &impl QueryExecutor,
        id: &EditorSessionId,
        remote_post: Option<RemotePostReference>,
        body: &str,
    ) -> Result<bool, LocalContentError> {
        let inserted = executor.execute(
            &format!(
                "INSERT OR REPLACE INTO {UNCONFIRMED_PUSHES}
                     (editor_session_rowid, remote_post_id, base_modified_gmt, sent_at, body)
                 SELECT rowid, ?, ?, {NOW}, ? FROM {EDITOR_SESSIONS} WHERE uuid = ?"
            ),
            rusqlite::params![
                remote_post.map(|remote| remote.post_id.0),
                remote_post.map(|remote| remote.base_modified_gmt.0.timestamp()),
                body,
                id,
            ],
        )?;
        Ok(inserted > 0)
    }

    /// Delete the session's unconfirmed push, if it has one.
    ///
    /// Returns `false` if no session has the ID.
    pub fn clear_unconfirmed_push(
        &self,
        executor: &impl QueryExecutor,
        id: &EditorSessionId,
    ) -> Result<bool, LocalContentError> {
        let Some(rowid) = select_rowid(executor, id)? else {
            return Ok(false);
        };
        executor.execute(
            &format!("DELETE FROM {UNCONFIRMED_PUSHES} WHERE editor_session_rowid = ?"),
            [rowid],
        )?;
        Ok(true)
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
    SELECT s.uuid, w.uuid, s.started_at, s.last_recorded_at, s.ended_at,
           p.remote_post_id, p.base_modified_gmt, p.sent_at, p.body
    FROM editor_sessions s
    JOIN working_copies w ON w.rowid = s.working_copy_rowid
    LEFT JOIN editor_session_unconfirmed_pushes p ON p.editor_session_rowid = s.rowid";

fn session_from_row(row: &Row) -> rusqlite::Result<EditorSession> {
    let date = |index: usize| -> rusqlite::Result<_> {
        row.get::<_, Option<i64>>(index)?
            .map(|seconds| {
                date_from_timestamp(seconds).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(index, Type::Integer, Box::new(e))
                })
            })
            .transpose()
    };
    let remote_post_id: Option<i64> = row.get(5)?;
    let unconfirmed_push = match date(7)? {
        Some(sent_at) => Some(UnconfirmedPush {
            remote_post: remote_post_id
                .zip(date(6)?)
                .map(|(post_id, base_modified_gmt)| RemotePostReference {
                    post_id: PostId(post_id),
                    base_modified_gmt,
                }),
            sent_at,
            body: row.get(8)?,
        }),
        None => None,
    };
    Ok(EditorSession {
        id: row.get(0)?,
        working_copy_id: row.get(1)?,
        started_at: date(2)?.ok_or(rusqlite::Error::InvalidColumnType(
            2,
            "started_at".to_string(),
            Type::Null,
        ))?,
        last_recorded_at: date(3)?,
        ended_at: date(4)?,
        unconfirmed_push,
    })
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

    fn select(conn: &Connection, id: &EditorSessionId) -> EditorSession {
        EditorSessionRepository
            .select(conn, id)
            .expect("Select should succeed")
            .expect("The session should exist")
    }

    #[test]
    fn test_session_records_what_happened_in_it() {
        let (conn, working_copy_id) = connection_with_working_copy();

        let id = EditorSessionRepository
            .insert(&conn, &working_copy_id)
            .expect("Insert should succeed");
        let started = select(&conn, &id);
        assert_eq!(started.working_copy_id, working_copy_id);
        assert_eq!(
            (
                started.last_recorded_at,
                started.ended_at,
                started.unconfirmed_push
            ),
            (None, None, None)
        );

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
        let ended = EditorSessionRepository
            .insert(&conn, &working_copy_id)
            .expect("Insert should succeed");
        let unfinished = EditorSessionRepository
            .insert(&conn, &working_copy_id)
            .expect("Insert should succeed");
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
    fn test_unconfirmed_push_is_replaced_and_cleared() {
        let (conn, working_copy_id) = connection_with_working_copy();
        let id = EditorSessionRepository
            .insert(&conn, &working_copy_id)
            .expect("Insert should succeed");
        let remote_post = RemotePostReference {
            post_id: PostId(42),
            base_modified_gmt: date_from_timestamp(1_700_000_000).expect("Timestamp is in range"),
        };

        EditorSessionRepository
            .record_unconfirmed_push(&conn, &id, None, r#"{"title":"Create"}"#)
            .expect("Recording should succeed");
        EditorSessionRepository
            .record_unconfirmed_push(&conn, &id, Some(remote_post), r#"{"title":"Update"}"#)
            .expect("Recording should succeed");
        let push = select(&conn, &id)
            .unconfirmed_push
            .expect("The push should be recorded");
        assert_eq!(push.remote_post, Some(remote_post));
        assert_eq!(push.body, r#"{"title":"Update"}"#);

        EditorSessionRepository
            .clear_unconfirmed_push(&conn, &id)
            .expect("Clearing should succeed");
        assert_eq!(select(&conn, &id).unconfirmed_push, None);
    }

    #[test]
    fn test_session_needs_an_existing_working_copy_and_goes_with_it() {
        let (conn, working_copy_id) = connection_with_working_copy();
        let missing = WorkingCopyId("not-a-working-copy".to_string());

        assert_eq!(
            EditorSessionRepository.insert(&conn, &missing),
            Err(LocalContentError::WorkingCopyNotFound { id: missing })
        );

        let id = EditorSessionRepository
            .insert(&conn, &working_copy_id)
            .expect("Insert should succeed");
        EditorSessionRepository
            .record_unconfirmed_push(&conn, &id, None, "{}")
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
    }
}
