//! Reader posts, streams, and saved posts state. Everything here belongs to a WordPress.com account
//! (see [`super::wordpress_com_accounts`]) and goes with it.

use crate::{
    RowId, SqliteDbError,
    db_types::reader::{
        DbReaderSavedLocalOnly, DbReaderSavedState, DbReaderStream, DbReaderStreamItem,
        ReaderPostRecord, reader_date_from_text, reader_date_text, reader_saved_key_from_text,
        reader_saved_key_text,
    },
    list_metadata::ListState,
    repository::QueryExecutor,
};
use rusqlite::{OptionalExtension, Row, params};
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_saved::{
    ReaderFeedId, ReaderFeedItemId, ReaderPostSaveIdentity, ReaderSavedKey,
};

/// The `reader_posts` columns [`ReaderPostRecord`] is read from, in the order [`post_from_row`]
/// reads them, prefixed with `p.`.
const POST_COLUMNS: &str = "p.rowid, p.identity_key, p.post_id, p.site_id, p.feed_id, \
    p.feed_item_id, p.is_external, p.url, p.global_id, p.title, p.excerpt, p.content, p.date, \
    p.modified, p.author_name, p.author_avatar_url, p.author_url, p.featured_image, p.site_name, \
    p.site_url, p.feed_url, p.like_count, p.i_like, p.is_following";
const POST_COLUMN_COUNT: usize = 24;

/// Reads a post from `POST_COLUMNS`, starting at column `offset`.
fn post_from_row(row: &Row, offset: usize) -> Result<(RowId, ReaderPostRecord), SqliteDbError> {
    let column = |index: usize| offset + index;
    let optional_date = |index: usize| -> Result<_, SqliteDbError> {
        row.get::<_, Option<String>>(column(index))?
            .map(|text| reader_date_from_text(&text))
            .transpose()
    };
    let identity_key: String = row.get(column(1))?;
    let record = ReaderPostRecord {
        key: reader_saved_key_from_text(&identity_key)?,
        identity: ReaderPostSaveIdentity {
            id: row.get(column(2))?,
            site_id: row.get::<_, Option<u64>>(column(3))?.map(WpComSiteId),
            feed_id: row.get::<_, Option<u64>>(column(4))?.map(ReaderFeedId),
            feed_item_id: row.get::<_, Option<u64>>(column(5))?.map(ReaderFeedItemId),
            is_external: row.get(column(6))?,
            url: row.get(column(7))?,
        },
        global_id: row.get(column(8))?,
        title: row.get(column(9))?,
        excerpt: row.get(column(10))?,
        content: row.get(column(11))?,
        date: optional_date(12)?,
        modified: optional_date(13)?,
        author_name: row.get(column(14))?,
        author_avatar_url: row.get(column(15))?,
        author_url: row.get(column(16))?,
        featured_image: row.get(column(17))?,
        site_name: row.get(column(18))?,
        site_url: row.get(column(19))?,
        feed_url: row.get(column(20))?,
        like_count: row.get(column(21))?,
        i_like: row.get(column(22))?,
        is_following: row.get(column(23))?,
    };
    Ok((row.get(column(0))?, record))
}

fn collect_rows<T>(
    rows: impl Iterator<Item = Result<T, SqliteDbError>>,
) -> Result<Vec<T>, SqliteDbError> {
    rows.collect()
}

/// Reader posts, one row per post per account.
pub struct ReaderPostRepository;

impl ReaderPostRepository {
    /// Stores a post under its key. A field the post doesn't have keeps what's stored, so a source
    /// that sends part of a post doesn't erase what another sent.
    pub fn upsert(
        executor: &impl QueryExecutor,
        account: RowId,
        post: &ReaderPostRecord,
    ) -> Result<RowId, SqliteDbError> {
        let identity_key = reader_saved_key_text(&post.key);
        executor.execute(
            "INSERT INTO reader_posts (
                account_id, identity_key, post_id, site_id, feed_id, feed_item_id, is_external,
                url, global_id, title, excerpt, content, date, modified, author_name,
                author_avatar_url, author_url, featured_image, site_name, site_url, feed_url,
                like_count, i_like, is_following
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21, ?22, ?23, ?24)
            ON CONFLICT(account_id, identity_key) DO UPDATE SET
                post_id = excluded.post_id,
                site_id = COALESCE(excluded.site_id, site_id),
                feed_id = COALESCE(excluded.feed_id, feed_id),
                feed_item_id = COALESCE(excluded.feed_item_id, feed_item_id),
                is_external = excluded.is_external,
                url = COALESCE(excluded.url, url),
                global_id = COALESCE(excluded.global_id, global_id),
                title = COALESCE(excluded.title, title),
                excerpt = COALESCE(excluded.excerpt, excerpt),
                content = COALESCE(excluded.content, content),
                date = COALESCE(excluded.date, date),
                modified = COALESCE(excluded.modified, modified),
                author_name = COALESCE(excluded.author_name, author_name),
                author_avatar_url = COALESCE(excluded.author_avatar_url, author_avatar_url),
                author_url = COALESCE(excluded.author_url, author_url),
                featured_image = COALESCE(excluded.featured_image, featured_image),
                site_name = COALESCE(excluded.site_name, site_name),
                site_url = COALESCE(excluded.site_url, site_url),
                feed_url = COALESCE(excluded.feed_url, feed_url),
                like_count = COALESCE(excluded.like_count, like_count),
                i_like = COALESCE(excluded.i_like, i_like),
                is_following = COALESCE(excluded.is_following, is_following),
                fetched_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            params![
                account,
                identity_key,
                post.identity.id,
                post.identity.site_id.map(|id| id.0),
                post.identity.feed_id.map(|id| id.0),
                post.identity.feed_item_id.map(|id| id.0),
                post.identity.is_external,
                post.identity.url,
                post.global_id,
                post.title,
                post.excerpt,
                post.content,
                post.date.as_ref().map(reader_date_text),
                post.modified.as_ref().map(reader_date_text),
                post.author_name,
                post.author_avatar_url,
                post.author_url,
                post.featured_image,
                post.site_name,
                post.site_url,
                post.feed_url,
                post.like_count,
                post.i_like,
                post.is_following,
            ],
        )?;
        let mut statement = executor.prepare(
            "SELECT rowid FROM reader_posts WHERE account_id = ?1 AND identity_key = ?2",
        )?;
        Ok(statement.query_row(params![account, identity_key], |row| row.get(0))?)
    }

    /// The post stored under a key. A feed item key also finds a WordPress.com post that has that
    /// feed item, because the server stores a save under either.
    pub fn select_by_key(
        executor: &impl QueryExecutor,
        account: RowId,
        key: &ReaderSavedKey,
    ) -> Result<Option<(RowId, ReaderPostRecord)>, SqliteDbError> {
        let sql = format!(
            "SELECT {POST_COLUMNS} FROM reader_posts p WHERE p.account_id = ?1 AND p.identity_key = ?2"
        );
        let mut statement = executor.prepare(&sql)?;
        let exact = statement
            .query_row(params![account, reader_saved_key_text(key)], |row| {
                Ok(post_from_row(row, 0))
            })
            .optional()?
            .transpose()?;
        if exact.is_some() {
            return Ok(exact);
        }

        let ReaderSavedKey::FeedItem { feed_item_id } = key else {
            return Ok(None);
        };
        let sql = format!(
            "SELECT {POST_COLUMNS} FROM reader_posts p
            WHERE p.account_id = ?1 AND p.feed_item_id = ?2
            ORDER BY p.rowid LIMIT 1"
        );
        let mut statement = executor.prepare(&sql)?;
        Ok(statement
            .query_row(params![account, feed_item_id.0], |row| {
                Ok(post_from_row(row, 0))
            })
            .optional()?
            .transpose()?)
    }
}

/// Reader streams, such as the saved list, and the posts in them.
pub struct ReaderStreamRepository;

impl ReaderStreamRepository {
    const STREAM_COLUMNS: &str =
        "rowid, next_page_handle, has_more, last_refreshed_at, state, error_message, version";

    fn stream_from_row(row: &Row) -> Result<DbReaderStream, SqliteDbError> {
        Ok(DbReaderStream {
            row_id: row.get(0)?,
            next_page_handle: row.get(1)?,
            has_more: row.get(2)?,
            last_refreshed_at: row
                .get::<_, Option<String>>(3)?
                .map(|text| reader_date_from_text(&text))
                .transpose()?,
            state: row.get(4)?,
            error_message: row.get(5)?,
            version: row.get(6)?,
        })
    }

    /// The stream, if it's been created.
    pub fn select(
        executor: &impl QueryExecutor,
        account: RowId,
        key: &str,
    ) -> Result<Option<DbReaderStream>, SqliteDbError> {
        let sql = format!(
            "SELECT {} FROM reader_streams WHERE account_id = ?1 AND key = ?2",
            Self::STREAM_COLUMNS
        );
        let mut statement = executor.prepare(&sql)?;
        statement
            .query_row(params![account, key], |row| Ok(Self::stream_from_row(row)))
            .optional()?
            .transpose()
    }

    pub fn get_or_create(
        executor: &impl QueryExecutor,
        account: RowId,
        key: &str,
    ) -> Result<DbReaderStream, SqliteDbError> {
        executor.execute(
            "INSERT INTO reader_streams (account_id, key) VALUES (?1, ?2)
            ON CONFLICT(account_id, key) DO NOTHING",
            params![account, key],
        )?;
        Self::select(executor, account, key)?
            .ok_or_else(|| SqliteDbError::SqliteError(format!("Reader stream {key} wasn't stored")))
    }

    /// Starts a refresh: marks the stream as fetching its first page, and bumps its version so a
    /// load-more already in flight is discarded. Returns the new version.
    pub fn begin_refresh(
        executor: &impl QueryExecutor,
        stream: RowId,
    ) -> Result<i64, SqliteDbError> {
        executor.execute(
            "UPDATE reader_streams SET version = version + 1, state = ?2, error_message = NULL
            WHERE rowid = ?1",
            params![stream, ListState::FetchingFirstPage],
        )?;
        let mut statement =
            executor.prepare("SELECT version FROM reader_streams WHERE rowid = ?1")?;
        Ok(statement.query_row([stream], |row| row.get(0))?)
    }

    pub fn set_state(
        executor: &impl QueryExecutor,
        stream: RowId,
        state: ListState,
        error_message: Option<&str>,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE reader_streams SET state = ?2, error_message = ?3 WHERE rowid = ?1",
            params![stream, state, error_message],
        )?;
        Ok(())
    }

    /// Records where the stream is up to after a page loaded, and that it's idle. `refreshed` is
    /// whether the page was the first.
    pub fn set_cursor(
        executor: &impl QueryExecutor,
        stream: RowId,
        next_page_handle: Option<&str>,
        refreshed: bool,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE reader_streams SET
                next_page_handle = ?2,
                has_more = ?3,
                last_refreshed_at = CASE WHEN ?4
                    THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now') ELSE last_refreshed_at END,
                state = ?5,
                error_message = NULL
            WHERE rowid = ?1",
            params![
                stream,
                next_page_handle,
                next_page_handle.is_some(),
                refreshed,
                ListState::Idle
            ],
        )?;
        Ok(())
    }

    /// Adds a post to the stream, or moves it to `sort_key` if it's there already.
    pub fn upsert_item(
        executor: &impl QueryExecutor,
        stream: RowId,
        post: RowId,
        sort_key: &str,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "INSERT INTO reader_stream_items (stream_id, post_id, sort_key) VALUES (?1, ?2, ?3)
            ON CONFLICT(stream_id, post_id) DO UPDATE SET sort_key = excluded.sort_key",
            params![stream, post, sort_key],
        )?;
        Ok(())
    }

    pub fn remove_item(
        executor: &impl QueryExecutor,
        stream: RowId,
        post: RowId,
    ) -> Result<bool, SqliteDbError> {
        let removed = executor.execute(
            "DELETE FROM reader_stream_items WHERE stream_id = ?1 AND post_id = ?2",
            params![stream, post],
        )?;
        Ok(removed > 0)
    }

    /// The stream's posts, newest first.
    pub fn select_items(
        executor: &impl QueryExecutor,
        stream: RowId,
    ) -> Result<Vec<DbReaderStreamItem>, SqliteDbError> {
        let sql = format!(
            "SELECT {POST_COLUMNS}, i.sort_key FROM reader_stream_items i
            JOIN reader_posts p ON p.rowid = i.post_id
            WHERE i.stream_id = ?1
            ORDER BY i.sort_key DESC, i.rowid ASC"
        );
        let mut statement = executor.prepare(&sql)?;
        let rows = statement.query_map([stream], |row| {
            Ok(post_from_row(row, 0).and_then(|(post_row_id, post)| {
                Ok(DbReaderStreamItem {
                    post,
                    post_row_id,
                    sort_key: row.get(POST_COLUMN_COUNT)?,
                })
            }))
        })?;
        collect_rows(rows.map(|row| row.map_err(SqliteDbError::from).and_then(|row| row)))
    }

    /// Resets streams left fetching when the app quit. Called after migrations.
    pub fn reset_stale_fetching_states(
        executor: &impl QueryExecutor,
    ) -> Result<usize, SqliteDbError> {
        executor.execute(
            "UPDATE reader_streams SET state = ?1 WHERE state IN (?2, ?3)",
            params![
                ListState::Idle,
                ListState::FetchingFirstPage,
                ListState::FetchingNextPage
            ],
        )
    }
}

/// What the saved posts service keeps across launches.
pub struct ReaderSavedRepository;

impl ReaderSavedRepository {
    pub fn select_state(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<DbReaderSavedState, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT snapshot, local, imported_local_saves FROM reader_saved_state
            WHERE account_id = ?1",
        )?;
        Ok(statement
            .query_row([account], |row| {
                Ok(DbReaderSavedState {
                    snapshot: row.get(0)?,
                    local: row.get(1)?,
                    imported_local_saves: row.get(2)?,
                })
            })
            .optional()?
            .unwrap_or_default())
    }

    pub fn write_local(
        executor: &impl QueryExecutor,
        account: RowId,
        local: &str,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "INSERT INTO reader_saved_state (account_id, local) VALUES (?1, ?2)
            ON CONFLICT(account_id) DO UPDATE SET local = excluded.local",
            params![account, local],
        )?;
        Ok(())
    }

    pub fn write_snapshot(
        executor: &impl QueryExecutor,
        account: RowId,
        snapshot: &str,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "INSERT INTO reader_saved_state (account_id, snapshot) VALUES (?1, ?2)
            ON CONFLICT(account_id) DO UPDATE SET snapshot = excluded.snapshot",
            params![account, snapshot],
        )?;
        Ok(())
    }

    pub fn set_imported_local_saves(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "INSERT INTO reader_saved_state (account_id, imported_local_saves) VALUES (?1, 1)
            ON CONFLICT(account_id) DO UPDATE SET imported_local_saves = 1",
            [account],
        )?;
        Ok(())
    }

    pub fn upsert_local_only(
        executor: &impl QueryExecutor,
        account: RowId,
        key: &ReaderSavedKey,
        post: Option<RowId>,
        date_saved: Option<&str>,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "INSERT INTO reader_saved_local_only (account_id, key, post_id, date_saved)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(account_id, key) DO UPDATE SET
                post_id = COALESCE(excluded.post_id, post_id),
                date_saved = COALESCE(excluded.date_saved, date_saved)",
            params![account, reader_saved_key_text(key), post, date_saved],
        )?;
        Ok(())
    }

    pub fn delete_local_only(
        executor: &impl QueryExecutor,
        account: RowId,
        key: &ReaderSavedKey,
    ) -> Result<bool, SqliteDbError> {
        let deleted = executor.execute(
            "DELETE FROM reader_saved_local_only WHERE account_id = ?1 AND key = ?2",
            params![account, reader_saved_key_text(key)],
        )?;
        Ok(deleted > 0)
    }

    /// Local-only saves, newest first, with their posts when stored.
    pub fn select_local_only(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<Vec<DbReaderSavedLocalOnly>, SqliteDbError> {
        let sql = format!(
            "SELECT l.key, l.date_saved, l.post_id, {POST_COLUMNS} FROM reader_saved_local_only l
            LEFT JOIN reader_posts p ON p.rowid = l.post_id
            WHERE l.account_id = ?1
            ORDER BY l.date_saved IS NULL, l.date_saved DESC, l.rowid ASC"
        );
        let mut statement = executor.prepare(&sql)?;
        let rows = statement.query_map([account], |row| {
            Ok((|| -> Result<DbReaderSavedLocalOnly, SqliteDbError> {
                let key: String = row.get(0)?;
                let date_saved: Option<String> = row.get(1)?;
                let post_row: Option<RowId> = row.get(2)?;
                Ok(DbReaderSavedLocalOnly {
                    key: reader_saved_key_from_text(&key)?,
                    date_saved: date_saved
                        .map(|text| reader_date_from_text(&text))
                        .transpose()?,
                    post: match post_row {
                        Some(_) => Some(post_from_row(row, 3)?.1),
                        None => None,
                    },
                })
            })())
        })?;
        collect_rows(rows.map(|row| row.map_err(SqliteDbError::from).and_then(|row| row)))
    }
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
