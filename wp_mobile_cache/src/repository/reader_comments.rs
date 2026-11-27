//! Posts' comments, and the comments a reader wrote on this device that the server hasn't taken.

use crate::{
    RowId, SqliteDbError,
    db_types::reader::{reader_date_from_text, reader_date_text},
    list_metadata::ListState,
    repository::QueryExecutor,
};
use rusqlite::{OptionalExtension, Row, params};
use wp_api::comments::CommentId;
use wp_api::date::WpGmtDateTime;
use wp_api::posts::PostId;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_comments::{ReaderCommentStatus, ReaderCommentType};

/// Where a comment written on this device is on its way to the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ReaderCommentSendState {
    /// The server has it. Every comment the server sent is this.
    Sent,
    /// Written here, and being sent.
    Sending,
    /// Written here, and not taken: the server refused it, or no answer came. It stays until the
    /// reader sends it again or discards it.
    Failed,
}

/// A comment, as the cache has it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderCommentRecord {
    pub site_id: WpComSiteId,
    pub post_id: PostId,
    /// `None` for a comment written on this device that the server hasn't taken.
    #[uniffi(default = None)]
    pub comment_id: Option<CommentId>,
    /// Set for a comment written on this device: what to call it until it has the server's ID.
    #[uniffi(default = None)]
    pub local_id: Option<String>,
    /// The comment it replies to. `None` for a comment on the post itself.
    #[uniffi(default = None)]
    pub parent_id: Option<CommentId>,
    /// `0` for a guest or a pingback.
    #[uniffi(default = None)]
    pub author_id: Option<u64>,
    #[uniffi(default = None)]
    pub author_name: Option<String>,
    #[uniffi(default = None)]
    pub author_avatar_url: Option<String>,
    #[uniffi(default = None)]
    pub author_url: Option<String>,
    #[uniffi(default = None)]
    pub date: Option<WpGmtDateTime>,
    /// HTML, as the site sent it. For a comment written here, what the reader typed.
    #[uniffi(default = None)]
    pub content: Option<String>,
    #[uniffi(default = None)]
    pub status: Option<ReaderCommentStatus>,
    #[uniffi(default = None)]
    pub comment_type: Option<ReaderCommentType>,
    #[uniffi(default = None)]
    pub like_count: Option<u64>,
    #[uniffi(default = None)]
    pub i_like: Option<bool>,
    pub send_state: ReaderCommentSendState,
    /// Why a comment written here wasn't taken.
    #[uniffi(default = None)]
    pub send_error: Option<String>,
}

/// Whether the user likes a comment, and how many likes it has, as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DbReaderCommentLike {
    pub i_like: Option<bool>,
    pub like_count: Option<u64>,
}

/// Where a post's comments are up to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbReaderCommentThread {
    pub row_id: RowId,
    /// How many comments the server has, as of the newest page.
    pub found: Option<u64>,
    pub last_refreshed_at: Option<WpGmtDateTime>,
    pub state: ListState,
    pub error_message: Option<String>,
}

fn status_text(status: &ReaderCommentStatus) -> &str {
    match status {
        ReaderCommentStatus::Approved => "approved",
        ReaderCommentStatus::Unapproved => "unapproved",
        ReaderCommentStatus::Spam => "spam",
        ReaderCommentStatus::Trash => "trash",
        ReaderCommentStatus::Other(status) => status,
    }
}

fn status_from_text(status: String) -> ReaderCommentStatus {
    match status.as_str() {
        "approved" => ReaderCommentStatus::Approved,
        "unapproved" => ReaderCommentStatus::Unapproved,
        "spam" => ReaderCommentStatus::Spam,
        "trash" => ReaderCommentStatus::Trash,
        _ => ReaderCommentStatus::Other(status),
    }
}

fn type_text(comment_type: &ReaderCommentType) -> &str {
    match comment_type {
        ReaderCommentType::Comment => "comment",
        ReaderCommentType::Pingback => "pingback",
        ReaderCommentType::Trackback => "trackback",
        ReaderCommentType::Other(comment_type) => comment_type,
    }
}

fn type_from_text(comment_type: String) -> ReaderCommentType {
    match comment_type.as_str() {
        "comment" => ReaderCommentType::Comment,
        "pingback" => ReaderCommentType::Pingback,
        "trackback" => ReaderCommentType::Trackback,
        _ => ReaderCommentType::Other(comment_type),
    }
}

fn send_state_number(state: ReaderCommentSendState) -> i64 {
    match state {
        ReaderCommentSendState::Sent => 0,
        ReaderCommentSendState::Sending => 1,
        ReaderCommentSendState::Failed => 2,
    }
}

/// The rows of a post's comments that are hidden because of a blocked user, as a `hidden(rowid)`
/// table: a comment written by a user in `reader_blocked_users`, and every reply under one, at any
/// depth. A comment written on this device that the server hasn't taken isn't among them, so it
/// can still be sent again or discarded. Takes the account, site and post as `?1`, `?2` and `?3`.
const HIDDEN_COMMENTS: &str = "WITH RECURSIVE hidden(rowid, comment_id) AS (
        SELECT c.rowid, c.comment_id FROM reader_comments c
        JOIN reader_blocked_users u ON u.account_id = c.account_id AND u.user_id = c.author_id
        WHERE c.account_id = ?1 AND c.site_id = ?2 AND c.post_id = ?3 AND c.local_id IS NULL
        UNION
        SELECT c.rowid, c.comment_id FROM reader_comments c
        JOIN hidden h ON c.parent_id = h.comment_id
        WHERE c.account_id = ?1 AND c.site_id = ?2 AND c.post_id = ?3 AND c.local_id IS NULL
    )";

const COMMENT_COLUMNS: &str = "site_id, post_id, comment_id, local_id, parent_id, author_id, \
    author_name, author_avatar_url, author_url, date, content, status, comment_type, like_count, \
    i_like, send_state, send_error";

fn comment_from_row(row: &Row) -> Result<ReaderCommentRecord, SqliteDbError> {
    Ok(ReaderCommentRecord {
        site_id: WpComSiteId(row.get(0)?),
        post_id: PostId(row.get(1)?),
        comment_id: row.get::<_, Option<i64>>(2)?.map(CommentId),
        local_id: row.get(3)?,
        parent_id: row.get::<_, Option<i64>>(4)?.map(CommentId),
        author_id: row.get(5)?,
        author_name: row.get(6)?,
        author_avatar_url: row.get(7)?,
        author_url: row.get(8)?,
        date: row
            .get::<_, Option<String>>(9)?
            .map(|text| reader_date_from_text(&text))
            .transpose()?,
        content: row.get(10)?,
        status: row.get::<_, Option<String>>(11)?.map(status_from_text),
        comment_type: row.get::<_, Option<String>>(12)?.map(type_from_text),
        like_count: row.get(13)?,
        i_like: row.get(14)?,
        send_state: match row.get::<_, i64>(15)? {
            1 => ReaderCommentSendState::Sending,
            2 => ReaderCommentSendState::Failed,
            _ => ReaderCommentSendState::Sent,
        },
        send_error: row.get(16)?,
    })
}

/// Posts' comments.
pub struct ReaderCommentRepository;

impl ReaderCommentRepository {
    fn insert(
        executor: &impl QueryExecutor,
        account: RowId,
        comment: &ReaderCommentRecord,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "INSERT INTO reader_comments (
                account_id, site_id, post_id, comment_id, local_id, parent_id, author_id,
                author_name, author_avatar_url, author_url, date, content, status, comment_type,
                like_count, i_like, send_state, send_error
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                ?18)",
            params![
                account,
                comment.site_id.0,
                comment.post_id.0,
                comment.comment_id.map(|id| id.0),
                comment.local_id,
                comment.parent_id.map(|id| id.0),
                comment.author_id,
                comment.author_name,
                comment.author_avatar_url,
                comment.author_url,
                comment.date.as_ref().map(reader_date_text),
                comment.content,
                comment.status.as_ref().map(status_text),
                comment.comment_type.as_ref().map(type_text),
                comment.like_count,
                comment.i_like,
                send_state_number(comment.send_state),
                comment.send_error,
            ],
        )?;
        Ok(())
    }

    /// Stores a comment the server sent, in place of what's stored for its ID.
    pub fn upsert(
        executor: &impl QueryExecutor,
        account: RowId,
        comment: &ReaderCommentRecord,
    ) -> Result<(), SqliteDbError> {
        if let Some(id) = comment.comment_id {
            executor.execute(
                "DELETE FROM reader_comments
                WHERE account_id = ?1 AND site_id = ?2 AND comment_id = ?3",
                params![account, comment.site_id.0, id.0],
            )?;
        }
        Self::insert(executor, account, comment)
    }

    /// Makes a post's comments from the server be exactly `comments`. Comments written here that
    /// the server hasn't taken stay. Call it inside a transaction.
    pub fn replace_post(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: WpComSiteId,
        post_id: PostId,
        comments: &[ReaderCommentRecord],
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "DELETE FROM reader_comments
            WHERE account_id = ?1 AND site_id = ?2 AND post_id = ?3 AND send_state = 0",
            params![account, site_id.0, post_id.0],
        )?;
        for comment in comments {
            Self::upsert(executor, account, comment)?;
        }
        Ok(())
    }

    /// A post's comments, oldest first, with the ones written here that the server hasn't taken.
    ///
    /// A blocked user's comments are left out, with every reply under them: see
    /// [`Self::count_hidden`]. They stay stored, so an unblock shows them again.
    pub fn select_post(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: WpComSiteId,
        post_id: PostId,
    ) -> Result<Vec<ReaderCommentRecord>, SqliteDbError> {
        let sql = format!(
            "{HIDDEN_COMMENTS}
            SELECT {COMMENT_COLUMNS} FROM reader_comments
            WHERE account_id = ?1 AND site_id = ?2 AND post_id = ?3
                AND rowid NOT IN (SELECT rowid FROM hidden)
            ORDER BY date IS NULL, date, rowid"
        );
        let mut statement = executor.prepare(&sql)?;
        let rows = statement.query_map(params![account, site_id.0, post_id.0], |row| {
            Ok(comment_from_row(row))
        })?;
        rows.map(|row| row.map_err(SqliteDbError::from).and_then(|row| row))
            .collect()
    }

    /// How many of a post's stored comments [`Self::select_post`] leaves out because of a blocked
    /// user: the user's own, and the replies under them.
    pub fn count_hidden(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: WpComSiteId,
        post_id: PostId,
    ) -> Result<u64, SqliteDbError> {
        let sql = format!("{HIDDEN_COMMENTS} SELECT COUNT(*) FROM hidden");
        let mut statement = executor.prepare(&sql)?;
        Ok(statement.query_row(params![account, site_id.0, post_id.0], |row| row.get(0))?)
    }

    /// How many of a post's comments from the server are stored.
    pub fn count_sent(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: WpComSiteId,
        post_id: PostId,
    ) -> Result<u64, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT COUNT(*) FROM reader_comments
            WHERE account_id = ?1 AND site_id = ?2 AND post_id = ?3 AND send_state = 0",
        )?;
        Ok(statement.query_row(params![account, site_id.0, post_id.0], |row| row.get(0))?)
    }

    /// A comment written here, by its local ID.
    pub fn select_local(
        executor: &impl QueryExecutor,
        account: RowId,
        local_id: &str,
    ) -> Result<Option<ReaderCommentRecord>, SqliteDbError> {
        let sql = format!(
            "SELECT {COMMENT_COLUMNS} FROM reader_comments WHERE account_id = ?1 AND local_id = ?2"
        );
        let mut statement = executor.prepare(&sql)?;
        statement
            .query_row(params![account, local_id], |row| Ok(comment_from_row(row)))
            .optional()?
            .transpose()
    }

    /// Records where a comment written here is on its way: being sent, or not taken and why.
    pub fn set_send_state(
        executor: &impl QueryExecutor,
        account: RowId,
        local_id: &str,
        state: ReaderCommentSendState,
        error: Option<&str>,
    ) -> Result<bool, SqliteDbError> {
        Ok(executor.execute(
            "UPDATE reader_comments SET send_state = ?3, send_error = ?4
            WHERE account_id = ?1 AND local_id = ?2",
            params![account, local_id, send_state_number(state), error],
        )? > 0)
    }

    /// Removes a comment written here. Returns whether it was there.
    pub fn delete_local(
        executor: &impl QueryExecutor,
        account: RowId,
        local_id: &str,
    ) -> Result<bool, SqliteDbError> {
        Ok(executor.execute(
            "DELETE FROM reader_comments WHERE account_id = ?1 AND local_id = ?2",
            params![account, local_id],
        )? > 0)
    }

    /// Comments written here that were being sent when the app quit: their answer never arrived,
    /// so they're marked as not taken. Called after migrations.
    pub fn fail_comments_left_sending(
        executor: &impl QueryExecutor,
    ) -> Result<usize, SqliteDbError> {
        executor.execute(
            "UPDATE reader_comments
            SET send_state = 2, send_error = 'The app closed before an answer arrived.'
            WHERE send_state = 1",
            [],
        )
    }

    /// Whether the user likes a comment, and how many likes it has, or `None` for a comment the
    /// cache doesn't have.
    pub fn select_like(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: WpComSiteId,
        comment_id: CommentId,
    ) -> Result<Option<DbReaderCommentLike>, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT i_like, like_count FROM reader_comments
            WHERE account_id = ?1 AND site_id = ?2 AND comment_id = ?3",
        )?;
        Ok(statement
            .query_row(params![account, site_id.0, comment_id.0], |row| {
                Ok(DbReaderCommentLike {
                    i_like: row.get(0)?,
                    like_count: row.get(1)?,
                })
            })
            .optional()?)
    }

    /// Records whether the user likes a comment, and its like count when it's known.
    pub fn set_like(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: WpComSiteId,
        comment_id: CommentId,
        i_like: bool,
        like_count: Option<u64>,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE reader_comments SET i_like = ?4, like_count = COALESCE(?5, like_count)
            WHERE account_id = ?1 AND site_id = ?2 AND comment_id = ?3",
            params![account, site_id.0, comment_id.0, i_like, like_count],
        )?;
        Ok(())
    }
}

/// Where each post's comments are up to.
pub struct ReaderCommentThreadRepository;

impl ReaderCommentThreadRepository {
    pub fn select(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: WpComSiteId,
        post_id: PostId,
    ) -> Result<Option<DbReaderCommentThread>, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT rowid, found, last_refreshed_at, state, error_message
            FROM reader_comment_threads WHERE account_id = ?1 AND site_id = ?2 AND post_id = ?3",
        )?;
        statement
            .query_row(params![account, site_id.0, post_id.0], |row| {
                Ok((|| -> Result<DbReaderCommentThread, SqliteDbError> {
                    Ok(DbReaderCommentThread {
                        row_id: row.get(0)?,
                        found: row.get(1)?,
                        last_refreshed_at: row
                            .get::<_, Option<String>>(2)?
                            .map(|text| reader_date_from_text(&text))
                            .transpose()?,
                        state: row.get(3)?,
                        error_message: row.get(4)?,
                    })
                })())
            })
            .optional()?
            .transpose()
    }

    pub fn get_or_create(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: WpComSiteId,
        post_id: PostId,
    ) -> Result<DbReaderCommentThread, SqliteDbError> {
        executor.execute(
            "INSERT INTO reader_comment_threads (account_id, site_id, post_id)
            VALUES (?1, ?2, ?3) ON CONFLICT(account_id, site_id, post_id) DO NOTHING",
            params![account, site_id.0, post_id.0],
        )?;
        Self::select(executor, account, site_id, post_id)?.ok_or_else(|| {
            SqliteDbError::SqliteError("The Reader comment thread wasn't stored".to_string())
        })
    }

    pub fn set_state(
        executor: &impl QueryExecutor,
        thread: RowId,
        state: ListState,
        error_message: Option<&str>,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE reader_comment_threads SET state = ?2, error_message = ?3 WHERE rowid = ?1",
            params![thread, state, error_message],
        )?;
        Ok(())
    }

    /// Records that a page loaded: how many comments the server has, that the thread is idle,
    /// and when, if the page was the first.
    pub fn finish_page(
        executor: &impl QueryExecutor,
        thread: RowId,
        found: Option<u64>,
        refreshed_at: Option<&WpGmtDateTime>,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE reader_comment_threads SET
                found = COALESCE(?2, found),
                last_refreshed_at = COALESCE(?3, last_refreshed_at),
                state = ?4,
                error_message = NULL
            WHERE rowid = ?1",
            params![
                thread,
                found,
                refreshed_at.map(reader_date_text),
                ListState::Idle
            ],
        )?;
        Ok(())
    }

    /// Resets threads left fetching when the app quit. Called after migrations.
    pub fn reset_stale_fetching_states(
        executor: &impl QueryExecutor,
    ) -> Result<usize, SqliteDbError> {
        executor.execute(
            "UPDATE reader_comment_threads SET state = ?1 WHERE state IN (?2, ?3)",
            params![
                ListState::Idle,
                ListState::FetchingFirstPage,
                ListState::FetchingNextPage
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MigrationManager;
    use crate::repository::wordpress_com_accounts::WordPressComAccountRepository;
    use rusqlite::Connection;
    use wp_api::wp_com::me::WpComUserId;

    const SITE: WpComSiteId = WpComSiteId(10);
    const POST: PostId = PostId(20);

    fn conn() -> (Connection, RowId) {
        let conn = Connection::open_in_memory().unwrap();
        MigrationManager::new(&conn)
            .unwrap()
            .perform_migrations()
            .unwrap();
        let account = WordPressComAccountRepository::get_or_create(&conn, WpComUserId(1)).unwrap();
        (conn, account)
    }

    fn comment(id: i64, hour: u32) -> ReaderCommentRecord {
        ReaderCommentRecord {
            site_id: SITE,
            post_id: POST,
            comment_id: Some(CommentId(id)),
            local_id: None,
            parent_id: None,
            author_id: Some(7),
            author_name: Some("An author".to_string()),
            author_avatar_url: None,
            author_url: None,
            date: Some(format!("2026-11-18T{hour:02}:00:00Z").parse().unwrap()),
            content: Some(format!("<p>Comment {id}</p>")),
            status: Some(ReaderCommentStatus::Approved),
            comment_type: Some(ReaderCommentType::Comment),
            like_count: Some(0),
            i_like: Some(false),
            send_state: ReaderCommentSendState::Sent,
            send_error: None,
        }
    }

    fn written(local_id: &str, hour: u32) -> ReaderCommentRecord {
        ReaderCommentRecord {
            comment_id: None,
            local_id: Some(local_id.to_string()),
            status: None,
            send_state: ReaderCommentSendState::Sending,
            ..comment(0, hour)
        }
    }

    fn ids(conn: &Connection, account: RowId) -> Vec<String> {
        ReaderCommentRepository::select_post(conn, account, SITE, POST)
            .unwrap()
            .iter()
            .map(|comment| match (&comment.comment_id, &comment.local_id) {
                (Some(id), _) => id.0.to_string(),
                (None, Some(local)) => local.clone(),
                (None, None) => "?".to_string(),
            })
            .collect()
    }

    #[test]
    fn test_a_blocked_users_comments_and_the_replies_under_them_are_hidden_until_unblocked() {
        use crate::repository::reader_actions::ReaderBlockRepository;
        let (conn, account) = conn();
        let by = |id: i64, hour: u32, author: u64, parent: Option<i64>| ReaderCommentRecord {
            author_id: Some(author),
            parent_id: parent.map(CommentId),
            ..comment(id, hour)
        };
        // 2 is by the blocked user, 3 answers it, 4 answers 3, and 5 is on its own.
        for comment in [
            by(1, 10, 7, None),
            by(2, 11, 8, None),
            by(3, 12, 7, Some(2)),
            by(4, 13, 7, Some(3)),
            by(5, 14, 7, Some(1)),
        ] {
            ReaderCommentRepository::upsert(&conn, account, &comment).unwrap();
        }
        // A reply written here to the blocked user's comment, not taken yet.
        let reply = ReaderCommentRecord {
            parent_id: Some(CommentId(2)),
            ..written("local-1", 15)
        };
        ReaderCommentRepository::upsert(&conn, account, &reply).unwrap();

        ReaderBlockRepository::set_user_blocked(&conn, account, WpComUserId(8), true).unwrap();
        assert_eq!(ids(&conn, account), ["1", "5", "local-1"]);
        assert_eq!(
            ReaderCommentRepository::count_hidden(&conn, account, SITE, POST).unwrap(),
            3
        );
        // They're still stored: the server's count of what's cached doesn't change.
        assert_eq!(
            ReaderCommentRepository::count_sent(&conn, account, SITE, POST).unwrap(),
            5
        );

        ReaderBlockRepository::set_user_blocked(&conn, account, WpComUserId(8), false).unwrap();
        assert_eq!(ids(&conn, account), ["1", "2", "3", "4", "5", "local-1"]);
        assert_eq!(
            ReaderCommentRepository::count_hidden(&conn, account, SITE, POST).unwrap(),
            0
        );
    }

    #[test]
    fn test_comments_are_listed_oldest_first_and_stored_once() {
        let (conn, account) = conn();
        ReaderCommentRepository::upsert(&conn, account, &comment(2, 11)).unwrap();
        ReaderCommentRepository::upsert(&conn, account, &comment(1, 10)).unwrap();
        // The same comment again, edited.
        let edited = ReaderCommentRecord {
            content: Some("<p>Edited</p>".to_string()),
            ..comment(2, 11)
        };
        ReaderCommentRepository::upsert(&conn, account, &edited).unwrap();

        assert_eq!(ids(&conn, account), ["1", "2"]);
        let stored = ReaderCommentRepository::select_post(&conn, account, SITE, POST).unwrap();
        assert_eq!(stored[1], edited);
        assert_eq!(
            ReaderCommentRepository::count_sent(&conn, account, SITE, POST).unwrap(),
            2
        );
    }

    #[test]
    fn test_replacing_a_posts_comments_keeps_what_was_written_here() {
        let (conn, account) = conn();
        ReaderCommentRepository::upsert(&conn, account, &comment(1, 10)).unwrap();
        ReaderCommentRepository::upsert(&conn, account, &comment(2, 11)).unwrap();
        ReaderCommentRepository::upsert(&conn, account, &written("local-a", 12)).unwrap();

        // The server no longer has comment 1.
        ReaderCommentRepository::replace_post(
            &conn,
            account,
            SITE,
            POST,
            &[comment(2, 11), comment(3, 13)],
        )
        .unwrap();

        assert_eq!(ids(&conn, account), ["2", "local-a", "3"]);
        assert_eq!(
            ReaderCommentRepository::count_sent(&conn, account, SITE, POST).unwrap(),
            2
        );
    }

    #[test]
    fn test_a_comment_written_here_is_marked_and_removed_by_its_local_id() {
        let (conn, account) = conn();
        ReaderCommentRepository::upsert(&conn, account, &written("local-a", 12)).unwrap();
        assert!(
            ReaderCommentRepository::set_send_state(
                &conn,
                account,
                "local-a",
                ReaderCommentSendState::Failed,
                Some("offline"),
            )
            .unwrap()
        );
        let stored = ReaderCommentRepository::select_local(&conn, account, "local-a")
            .unwrap()
            .unwrap();
        assert_eq!(stored.send_state, ReaderCommentSendState::Failed);
        assert_eq!(stored.send_error.as_deref(), Some("offline"));

        assert!(ReaderCommentRepository::delete_local(&conn, account, "local-a").unwrap());
        assert!(!ReaderCommentRepository::delete_local(&conn, account, "local-a").unwrap());
        assert_eq!(
            ReaderCommentRepository::select_local(&conn, account, "local-a").unwrap(),
            None
        );
    }

    #[test]
    fn test_a_comment_left_sending_when_the_app_quit_is_marked_as_not_taken() {
        let (conn, account) = conn();
        ReaderCommentRepository::upsert(&conn, account, &written("local-a", 12)).unwrap();
        ReaderCommentRepository::upsert(&conn, account, &comment(1, 10)).unwrap();

        assert_eq!(
            ReaderCommentRepository::fail_comments_left_sending(&conn).unwrap(),
            1
        );

        let stored = ReaderCommentRepository::select_local(&conn, account, "local-a")
            .unwrap()
            .unwrap();
        assert_eq!(stored.send_state, ReaderCommentSendState::Failed);
        assert!(stored.send_error.is_some());
    }

    #[test]
    fn test_a_comments_like_is_read_and_set() {
        let (conn, account) = conn();
        ReaderCommentRepository::upsert(&conn, account, &comment(1, 10)).unwrap();
        let like = || {
            ReaderCommentRepository::select_like(&conn, account, SITE, CommentId(1))
                .unwrap()
                .map(|like| (like.i_like, like.like_count))
        };
        assert_eq!(like(), Some((Some(false), Some(0))));

        ReaderCommentRepository::set_like(&conn, account, SITE, CommentId(1), true, Some(1))
            .unwrap();
        assert_eq!(like(), Some((Some(true), Some(1))));
        // Without a count, the count stays.
        ReaderCommentRepository::set_like(&conn, account, SITE, CommentId(1), false, None).unwrap();
        assert_eq!(like(), Some((Some(false), Some(1))));
        assert_eq!(
            ReaderCommentRepository::select_like(&conn, account, SITE, CommentId(9)).unwrap(),
            None
        );
    }

    #[test]
    fn test_a_thread_keeps_how_many_comments_there_are() {
        let (conn, account) = conn();
        assert_eq!(
            ReaderCommentThreadRepository::select(&conn, account, SITE, POST).unwrap(),
            None
        );
        let thread =
            ReaderCommentThreadRepository::get_or_create(&conn, account, SITE, POST).unwrap();
        ReaderCommentThreadRepository::set_state(
            &conn,
            thread.row_id,
            ListState::FetchingFirstPage,
            None,
        )
        .unwrap();
        let now: WpGmtDateTime = "2026-11-18T10:00:00Z".parse().unwrap();
        ReaderCommentThreadRepository::finish_page(&conn, thread.row_id, Some(12), Some(&now))
            .unwrap();
        // A later page that doesn't say how many there are leaves the count.
        ReaderCommentThreadRepository::finish_page(&conn, thread.row_id, None, None).unwrap();

        let stored = ReaderCommentThreadRepository::select(&conn, account, SITE, POST)
            .unwrap()
            .unwrap();
        assert_eq!(stored.found, Some(12));
        assert_eq!(stored.last_refreshed_at, Some(now));
        assert_eq!(stored.state, ListState::Idle);

        ReaderCommentThreadRepository::set_state(
            &conn,
            thread.row_id,
            ListState::FetchingNextPage,
            None,
        )
        .unwrap();
        assert_eq!(
            ReaderCommentThreadRepository::reset_stale_fetching_states(&conn).unwrap(),
            1
        );
    }
}
