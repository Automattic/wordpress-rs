//! Reader posts, streams, and saved posts state. Everything here belongs to a WordPress.com account
//! (see [`super::wordpress_com_accounts`]) and goes with it.

use crate::{
    RowId, SqliteDbError,
    db_types::reader::{
        DbReaderSavedKey, DbReaderSavedListItem, DbReaderSavedRow, DbReaderSavedState,
        DbReaderStream, DbReaderStreamItem, ReaderPostRecord, reader_date_from_text,
        reader_date_text, reader_saved_key_from_text, reader_saved_key_text,
    },
    list_metadata::ListState,
    repository::QueryExecutor,
};
use rusqlite::{OptionalExtension, Row, params};
use wp_api::date::WpGmtDateTime;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_saved::{
    ReaderFeedId, ReaderFeedItemId, ReaderPostSaveIdentity, ReaderSavedKey,
};

/// The `reader_posts` columns [`ReaderPostRecord`] is read from, in the order [`post_from_row`]
/// reads them, prefixed with `p.`.
const POST_COLUMNS: &str = "p.rowid, p.identity_key, p.post_id, p.site_id, p.feed_id, \
    p.feed_item_id, p.is_external, p.url, p.global_id, p.title, p.excerpt, p.content, p.date, \
    p.modified, p.author_name, p.author_avatar_url, p.author_url, p.featured_image, p.site_name, \
    p.site_url, p.feed_url, p.like_count, p.i_like, p.is_following, p.site_icon_url, p.tags, \
    p.comment_count";
const POST_COLUMN_COUNT: usize = 27;

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
        site_icon_url: row.get(column(24))?,
        tags: row
            .get::<_, Option<String>>(column(25))?
            .map(|json| serde_json::from_str(&json))
            .transpose()
            .map_err(|error| {
                SqliteDbError::SqliteError(format!("Invalid Reader post tags: {error}"))
            })?,
        comment_count: row.get(column(26))?,
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

/// Which value a field keeps when a post is stored over one already stored, and both have it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Prefer {
    Incoming,
    Stored,
}

impl ReaderPostRepository {
    /// Stores a post under its key. A field the post doesn't have keeps what's stored, so a source
    /// that sends part of a post doesn't erase what another sent.
    pub fn upsert(
        executor: &impl QueryExecutor,
        account: RowId,
        post: &ReaderPostRecord,
    ) -> Result<RowId, SqliteDbError> {
        Self::store(executor, account, post, Prefer::Incoming)
    }

    /// Stores a post under its key, only filling in fields the stored post doesn't have. For details
    /// an app passes along, which may be older than what a server response already stored.
    pub fn fill_missing(
        executor: &impl QueryExecutor,
        account: RowId,
        post: &ReaderPostRecord,
    ) -> Result<RowId, SqliteDbError> {
        Self::store(executor, account, post, Prefer::Stored)
    }

    fn store(
        executor: &impl QueryExecutor,
        account: RowId,
        post: &ReaderPostRecord,
        prefer: Prefer,
    ) -> Result<RowId, SqliteDbError> {
        /// The columns a source may leave out.
        const OPTIONAL_COLUMNS: [&str; 23] = [
            "site_id",
            "feed_id",
            "feed_item_id",
            "url",
            "global_id",
            "title",
            "excerpt",
            "content",
            "date",
            "modified",
            "author_name",
            "author_avatar_url",
            "author_url",
            "featured_image",
            "site_name",
            "site_url",
            "feed_url",
            "like_count",
            "i_like",
            "is_following",
            "site_icon_url",
            "tags",
            "comment_count",
        ];
        let identity_key = reader_saved_key_text(&post.key);
        let tags = post
            .tags
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|error| {
                SqliteDbError::SqliteError(format!("Couldn't store Reader post tags: {error}"))
            })?;
        let mut updates: Vec<String> = Vec::new();
        if prefer == Prefer::Incoming {
            updates.push("post_id = excluded.post_id".to_string());
            updates.push("is_external = excluded.is_external".to_string());
        }
        for column in OPTIONAL_COLUMNS {
            updates.push(match prefer {
                Prefer::Incoming => format!("{column} = COALESCE(excluded.{column}, {column})"),
                Prefer::Stored => format!("{column} = COALESCE({column}, excluded.{column})"),
            });
        }
        if prefer == Prefer::Incoming {
            updates.push("fetched_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')".to_string());
        }
        let sql = format!(
            "INSERT INTO reader_posts (
                account_id, identity_key, post_id, site_id, feed_id, feed_item_id, is_external,
                url, global_id, title, excerpt, content, date, modified, author_name,
                author_avatar_url, author_url, featured_image, site_name, site_url, feed_url,
                like_count, i_like, is_following, site_icon_url, tags, comment_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27)
            ON CONFLICT(account_id, identity_key) DO UPDATE SET {}",
            updates.join(", ")
        );
        executor.execute(
            &sql,
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
                post.site_icon_url,
                tags,
                post.comment_count,
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
        statement
            .query_row(params![account, feed_item_id.0], |row| {
                Ok(post_from_row(row, 0))
            })
            .optional()?
            .transpose()
    }
}

impl ReaderPostRepository {
    /// The row of the post stored under a key, as [`Self::select_by_key`] finds it, without reading
    /// the post.
    pub fn select_row_id_by_key(
        executor: &impl QueryExecutor,
        account: RowId,
        key: &ReaderSavedKey,
    ) -> Result<Option<RowId>, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT rowid FROM reader_posts WHERE account_id = ?1 AND identity_key = ?2",
        )?;
        let exact = statement
            .query_row(params![account, reader_saved_key_text(key)], |row| {
                row.get(0)
            })
            .optional()?;
        if exact.is_some() {
            return Ok(exact);
        }
        let ReaderSavedKey::FeedItem { feed_item_id } = key else {
            return Ok(None);
        };
        let mut statement = executor.prepare(
            "SELECT rowid FROM reader_posts WHERE account_id = ?1 AND feed_item_id = ?2
            ORDER BY rowid LIMIT 1",
        )?;
        Ok(statement
            .query_row(params![account, feed_item_id.0], |row| row.get(0))
            .optional()?)
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

    /// Sets the stream's state, unless a refresh started since `version`. Returns whether it did.
    pub fn set_state_if_version(
        executor: &impl QueryExecutor,
        stream: RowId,
        version: i64,
        state: ListState,
        error_message: Option<&str>,
    ) -> Result<bool, SqliteDbError> {
        let updated = executor.execute(
            "UPDATE reader_streams SET state = ?3, error_message = ?4
            WHERE rowid = ?1 AND version = ?2",
            params![stream, version, state, error_message],
        )?;
        Ok(updated > 0)
    }

    /// Records where the stream is up to after a page loaded, and that it's idle. `refreshed` is
    /// whether the page was the first, which bumps the version again: a load-more that started
    /// during the refresh read the old cursor, so its page is discarded.
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
                version = version + CASE WHEN ?4 THEN 1 ELSE 0 END,
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

    /// Each item's post row and identity, in the order of [`Self::select_items`], without reading
    /// the rest of each post.
    pub fn select_item_identities(
        executor: &impl QueryExecutor,
        stream: RowId,
    ) -> Result<Vec<(RowId, ReaderPostSaveIdentity)>, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT p.rowid, p.post_id, p.site_id, p.feed_id, p.feed_item_id, p.is_external, p.url
            FROM reader_stream_items i
            JOIN reader_posts p ON p.rowid = i.post_id
            WHERE i.stream_id = ?1
            ORDER BY i.sort_key DESC, i.rowid ASC",
        )?;
        let rows = statement.query_map([stream], |row| {
            Ok((
                row.get(0)?,
                ReaderPostSaveIdentity {
                    id: row.get(1)?,
                    site_id: row.get::<_, Option<u64>>(2)?.map(WpComSiteId),
                    feed_id: row.get::<_, Option<u64>>(3)?.map(ReaderFeedId),
                    feed_item_id: row.get::<_, Option<u64>>(4)?.map(ReaderFeedItemId),
                    is_external: row.get(5)?,
                    url: row.get(6)?,
                },
            ))
        })?;
        rows.map(|row| row.map_err(SqliteDbError::from)).collect()
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

/// The saved posts of an account: one row per save on the server, as far as this device knows,
/// and what the user wants for it.
///
/// A row is pending while what the user wants differs from what the server last said. The service
/// sends pending rows, one at a time, until they agree. A row sent without an answer is in doubt:
/// the server may or may not have applied it, so it isn't sent again until a refresh of the keys
/// says what the server has.
///
/// Call the methods that change more than one row inside a transaction.
pub struct ReaderSavedRepository;

const ROW_COLUMNS: &str = "s.rowid, s.key, s.url_key, s.feed_item_key, s.saved, s.server, \
    s.in_doubt, s.seq, s.date_saved, s.post_id, s.listed, s.imported, s.waiting_for_room, s.tapped, \
    s.date_known";
const ROW_COLUMN_COUNT: usize = 15;

fn saved_row_from_row(row: &Row) -> Result<DbReaderSavedRow, SqliteDbError> {
    let key = |index: usize| -> Result<Option<ReaderSavedKey>, SqliteDbError> {
        row.get::<_, Option<String>>(index)?
            .map(|text| reader_saved_key_from_text(&text))
            .transpose()
    };
    Ok(DbReaderSavedRow {
        row_id: row.get(0)?,
        key: reader_saved_key_from_text(&row.get::<_, String>(1)?)?,
        url_key: key(2)?,
        feed_item_key: key(3)?,
        saved: row.get(4)?,
        server: row.get(5)?,
        in_doubt: row.get(6)?,
        seq: row.get(7)?,
        date_saved: row
            .get::<_, Option<String>>(8)?
            .map(|text| reader_date_from_text(&text))
            .transpose()?,
        post_row_id: row.get(9)?,
        listed: row.get(10)?,
        imported: row.get(11)?,
        waiting_for_room: row.get(12)?,
        tapped: row.get(13)?,
        date_known: row.get(14)?,
    })
}

/// The alias column a key goes in when the server stores it as another key.
fn alias_column(key: &ReaderSavedKey) -> Option<&'static str> {
    match key {
        ReaderSavedKey::Url { .. } => Some("url_key"),
        ReaderSavedKey::FeedItem { .. } => Some("feed_item_key"),
        ReaderSavedKey::Post { .. } => None,
    }
}

impl ReaderSavedRepository {
    pub fn select_state(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<DbReaderSavedState, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT keys_refreshed, keys_complete, total FROM reader_saved_state WHERE account_id = ?1",
        )?;
        Ok(statement
            .query_row([account], |row| {
                Ok(DbReaderSavedState {
                    keys_refreshed: row.get(0)?,
                    keys_complete: row.get(1)?,
                    total: row.get(2)?,
                })
            })
            .optional()?
            .unwrap_or_default())
    }

    pub fn set_total(
        executor: &impl QueryExecutor,
        account: RowId,
        total: u64,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "INSERT INTO reader_saved_state (account_id, total) VALUES (?1, ?2)
            ON CONFLICT(account_id) DO UPDATE SET total = excluded.total",
            params![account, total],
        )?;
        Ok(())
    }

    pub fn select_row(
        executor: &impl QueryExecutor,
        row_id: RowId,
    ) -> Result<Option<DbReaderSavedRow>, SqliteDbError> {
        let sql = format!("SELECT {ROW_COLUMNS} FROM reader_saved_posts s WHERE s.rowid = ?1");
        let mut statement = executor.prepare(&sql)?;
        statement
            .query_row([row_id], |row| Ok(saved_row_from_row(row)))
            .optional()?
            .transpose()
    }

    fn select_where(
        executor: &impl QueryExecutor,
        account: RowId,
        condition: &str,
        params: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<DbReaderSavedRow>, SqliteDbError> {
        let sql = format!(
            "SELECT {ROW_COLUMNS} FROM reader_saved_posts s WHERE s.account_id = ?1 AND ({condition})
            ORDER BY s.seq, s.rowid"
        );
        let mut statement = executor.prepare(&sql)?;
        let mut all: Vec<&dyn rusqlite::ToSql> = vec![&account];
        all.extend_from_slice(params);
        let rows = statement.query_map(all.as_slice(), |row| Ok(saved_row_from_row(row)))?;
        collect_rows(rows.map(|row| row.map_err(SqliteDbError::from).and_then(|row| row)))
    }

    /// The rows a post may be saved under, given its keys: a row stored under one of them, or one
    /// the server stores one of them as.
    pub fn select_rows_for_keys(
        executor: &impl QueryExecutor,
        account: RowId,
        keys: &[ReaderSavedKey],
    ) -> Result<Vec<DbReaderSavedRow>, SqliteDbError> {
        let mut rows: Vec<DbReaderSavedRow> = Vec::new();
        for key in keys {
            let text = reader_saved_key_text(key);
            let found = Self::select_where(
                executor,
                account,
                "s.key = ?2 OR s.url_key = ?2 OR s.feed_item_key = ?2",
                &[&text],
            )?;
            for row in found {
                if !rows.iter().any(|other| other.row_id == row.row_id) {
                    rows.push(row);
                }
            }
        }
        Ok(rows)
    }

    /// Rows to send, oldest tap first: pending or tapped, and not in doubt. Saves waiting for room
    /// are among them: the service decides when to try them.
    pub fn select_pending(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<Vec<DbReaderSavedRow>, SqliteDbError> {
        Self::select_where(
            executor,
            account,
            "s.in_doubt = 0 AND (s.tapped = 1 OR s.server IS NULL OR s.server != s.saved)",
            &[],
        )
    }

    fn next_seq(executor: &impl QueryExecutor, account: RowId) -> Result<i64, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM reader_saved_posts WHERE account_id = ?1",
        )?;
        Ok(statement.query_row([account], |row| row.get(0))?)
    }

    fn insert(
        executor: &impl QueryExecutor,
        account: RowId,
        key: &ReaderSavedKey,
        saved: bool,
        server: Option<bool>,
        seq: i64,
        listed: bool,
    ) -> Result<RowId, SqliteDbError> {
        executor.execute(
            "INSERT INTO reader_saved_posts (account_id, key, saved, server, seq, listed)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                account,
                reader_saved_key_text(key),
                saved,
                server,
                seq,
                listed
            ],
        )?;
        Ok(executor.last_insert_rowid())
    }

    fn delete(executor: &impl QueryExecutor, row_id: RowId) -> Result<(), SqliteDbError> {
        executor.execute("DELETE FROM reader_saved_posts WHERE rowid = ?1", [row_id])?;
        Ok(())
    }

    /// Records the other keys of a row, such as the feed item of a Jetpack post, where it hasn't
    /// got one.
    pub fn add_aliases(
        executor: &impl QueryExecutor,
        row_id: RowId,
        key: &ReaderSavedKey,
        aliases: &[ReaderSavedKey],
    ) -> Result<(), SqliteDbError> {
        let key = reader_saved_key_text(key);
        for alias in aliases {
            let text = reader_saved_key_text(alias);
            if text == key {
                continue;
            }
            if let Some(column) = alias_column(alias) {
                executor.execute(
                    &format!(
                        "UPDATE reader_saved_posts SET {column} = ?2
                        WHERE rowid = ?1 AND {column} IS NULL"
                    ),
                    params![row_id, text],
                )?;
            }
        }
        Ok(())
    }

    /// A tap of save. `main` is the key to save the post under, and `keys` every key it may be
    /// saved under. The post's row for `main` wants it saved, and leaves the list only once it's
    /// unsaved. A pending unsave of another of its rows is called off. Returns the row.
    pub fn tap_save(
        executor: &impl QueryExecutor,
        account: RowId,
        main: &ReaderSavedKey,
        keys: &[ReaderSavedKey],
        post: Option<RowId>,
        now: &WpGmtDateTime,
    ) -> Result<RowId, SqliteDbError> {
        let seq = Self::next_seq(executor, account)?;
        let primary = Self::select_rows_for_keys(executor, account, std::slice::from_ref(main))?
            .into_iter()
            .next();
        let row_id = match &primary {
            Some(row) => {
                executor.execute(
                    "UPDATE reader_saved_posts SET
                        date_known = CASE WHEN saved THEN date_known ELSE 0 END,
                        date_saved = CASE WHEN saved THEN date_saved ELSE ?3 END,
                        saved = 1, seq = ?2, listed = 1, tapped = 1
                    WHERE rowid = ?1",
                    params![row.row_id, seq, reader_date_text(now)],
                )?;
                row.row_id
            }
            None => {
                let row_id = Self::insert(executor, account, main, true, None, seq, true)?;
                executor.execute(
                    "UPDATE reader_saved_posts SET date_saved = ?2, tapped = 1 WHERE rowid = ?1",
                    params![row_id, reader_date_text(now)],
                )?;
                row_id
            }
        };
        executor.execute(
            "UPDATE reader_saved_posts SET post_id = COALESCE(post_id, ?2) WHERE rowid = ?1",
            params![row_id, post],
        )?;
        Self::add_aliases(executor, row_id, main, keys)?;
        for other in Self::select_rows_for_keys(executor, account, keys)? {
            if other.row_id == row_id || other.saved {
                continue;
            }
            match other.server {
                None => Self::delete(executor, other.row_id)?,
                Some(server) => {
                    executor.execute(
                        "UPDATE reader_saved_posts SET saved = ?2, tapped = 0 WHERE rowid = ?1",
                        params![other.row_id, server],
                    )?;
                }
            }
        }
        Ok(row_id)
    }

    /// A tap of unsave: every row the post may be saved under wants it unsaved, and a key without a
    /// row gets one, so every key is sent: the keys may be out of date. A save waiting for room that
    /// the server never had is removed, as there's nothing to unsave there. Returns the rows to wait
    /// for.
    pub fn tap_unsave(
        executor: &impl QueryExecutor,
        account: RowId,
        keys: &[ReaderSavedKey],
    ) -> Result<Vec<RowId>, SqliteDbError> {
        let seq = Self::next_seq(executor, account)?;
        let rows = Self::select_rows_for_keys(executor, account, keys)?;
        let mut waiting = Vec::new();
        for row in &rows {
            if row.waiting_for_room && row.server != Some(true) {
                Self::delete(executor, row.row_id)?;
                continue;
            }
            executor.execute(
                "UPDATE reader_saved_posts SET saved = 0, seq = ?2, tapped = 1 WHERE rowid = ?1",
                params![row.row_id, seq],
            )?;
            waiting.push(row.row_id);
        }
        for key in keys {
            let text = reader_saved_key_text(key);
            let matched = rows.iter().any(|row| {
                [
                    Some(&row.key),
                    // The server stores a URL as the post, but a feed item can be a separate,
                    // older save of it, so the feed item is sent too.
                    row.url_key.as_ref(),
                ]
                .into_iter()
                .flatten()
                .any(|other| reader_saved_key_text(other) == text)
            });
            if !matched {
                let row_id = Self::insert(executor, account, key, false, None, seq, false)?;
                executor.execute(
                    "UPDATE reader_saved_posts SET tapped = 1 WHERE rowid = ?1",
                    [row_id],
                )?;
                waiting.push(row_id);
            }
        }
        Ok(waiting)
    }

    pub fn set_in_doubt(
        executor: &impl QueryExecutor,
        row_id: RowId,
        in_doubt: bool,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE reader_saved_posts SET in_doubt = ?2 WHERE rowid = ?1",
            params![row_id, in_doubt],
        )?;
        Ok(())
    }

    /// Merges a row into another for the same save. The newer tap decides what the user wants,
    /// and the row kept says what the server has.
    fn merge(
        executor: &impl QueryExecutor,
        from: &DbReaderSavedRow,
        into: &DbReaderSavedRow,
    ) -> Result<(), SqliteDbError> {
        Self::add_aliases(
            executor,
            into.row_id,
            &into.key,
            &[
                Some(from.key.clone()),
                from.url_key.clone(),
                from.feed_item_key.clone(),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
        )?;
        executor.execute(
            "UPDATE reader_saved_posts SET
                saved = CASE WHEN ?2 > seq THEN ?3 ELSE saved END,
                seq = MAX(seq, ?2),
                listed = MAX(listed, ?4),
                waiting_for_room = MAX(waiting_for_room, ?5),
                tapped = MAX(tapped, ?8),
                date_known = CASE WHEN date_saved IS NULL THEN ?9 ELSE date_known END,
                date_saved = COALESCE(date_saved, ?6),
                post_id = COALESCE(post_id, ?7)
            WHERE rowid = ?1",
            params![
                into.row_id,
                from.seq,
                from.saved,
                from.listed,
                from.waiting_for_room,
                from.date_saved.as_ref().map(reader_date_text),
                from.post_row_id,
                from.tapped,
                from.date_known,
            ],
        )?;
        Self::delete(executor, from.row_id)
    }

    /// Moves a row to the key the server stores it as, merging it into that key's row if there is
    /// one. Returns the row it ends up in.
    fn rekey(
        executor: &impl QueryExecutor,
        account: RowId,
        row: &DbReaderSavedRow,
        stored: &ReaderSavedKey,
    ) -> Result<RowId, SqliteDbError> {
        let stored_text = reader_saved_key_text(stored);
        if reader_saved_key_text(&row.key) == stored_text {
            return Ok(row.row_id);
        }
        let existing = Self::select_where(executor, account, "s.key = ?2", &[&stored_text])?;
        match existing.into_iter().next() {
            Some(into) => {
                Self::merge(executor, row, &into)?;
                Ok(into.row_id)
            }
            None => {
                executor.execute(
                    "UPDATE reader_saved_posts SET key = ?2 WHERE rowid = ?1",
                    params![row.row_id, stored_text],
                )?;
                Self::add_aliases(executor, row.row_id, stored, std::slice::from_ref(&row.key))?;
                Ok(row.row_id)
            }
        }
    }

    /// Records the server's answer to a row it was sent: that `stored` is now `saved`. Returns
    /// the row, which is the stored key's.
    pub fn apply_answer(
        executor: &impl QueryExecutor,
        account: RowId,
        row_id: RowId,
        saved: bool,
        stored: &ReaderSavedKey,
        total: Option<u64>,
    ) -> Result<RowId, SqliteDbError> {
        if let Some(total) = total {
            Self::set_total(executor, account, total)?;
        }
        let Some(row) = Self::select_row(executor, row_id)? else {
            return Ok(row_id);
        };
        let row_id = Self::rekey(executor, account, &row, stored)?;
        executor.execute(
            "UPDATE reader_saved_posts SET server = ?2, in_doubt = 0, tapped = 0, waiting_for_room = 0
            WHERE rowid = ?1",
            params![row_id, saved],
        )?;
        Ok(row_id)
    }

    /// Records that the server refused what a row asked for: it's undone.
    ///
    /// `sent_seq` is the row's [`DbReaderSavedRow::seq`] when it was sent. A tap made since is
    /// newer than what was refused, so it stands, and is sent next. Returns whether the refusal
    /// undid anything.
    pub fn apply_refusal(
        executor: &impl QueryExecutor,
        row_id: RowId,
        sent_seq: i64,
    ) -> Result<bool, SqliteDbError> {
        let Some(row) = Self::select_row(executor, row_id)? else {
            return Ok(false);
        };
        if row.seq != sent_seq {
            Self::set_in_doubt(executor, row_id, false)?;
            return Ok(false);
        }
        match row.server {
            None => Self::delete(executor, row_id)?,
            Some(server) => {
                executor.execute(
                    "UPDATE reader_saved_posts SET saved = ?2, in_doubt = 0, tapped = 0
                    WHERE rowid = ?1",
                    params![row_id, server],
                )?;
            }
        }
        Ok(true)
    }

    /// Records that the server refused a save for the account's save cap, with no tap waiting to
    /// hear it. The save stays, and is sent again once there's room.
    ///
    /// `sent_seq` is as for [`Self::apply_refusal`]: a tap made since stands. Returns whether the
    /// row now waits.
    pub fn set_waiting_for_room(
        executor: &impl QueryExecutor,
        row_id: RowId,
        sent_seq: i64,
    ) -> Result<bool, SqliteDbError> {
        let Some(row) = Self::select_row(executor, row_id)? else {
            return Ok(false);
        };
        if row.seq != sent_seq || !row.saved {
            Self::set_in_doubt(executor, row_id, false)?;
            return Ok(false);
        }
        executor.execute(
            "UPDATE reader_saved_posts SET waiting_for_room = 1, in_doubt = 0, tapped = 0
            WHERE rowid = ?1",
            [row_id],
        )?;
        Ok(true)
    }

    /// How many saves wait for room under the account's save cap.
    pub fn count_waiting_for_room(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<u64, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT COUNT(*) FROM reader_saved_posts
            WHERE account_id = ?1 AND saved = 1 AND waiting_for_room = 1",
        )?;
        Ok(statement.query_row([account], |row| row.get(0))?)
    }

    /// Applies a refresh of the saved keys: newest first, `returned` of them before any normalized
    /// to the same key, out of `total`.
    ///
    /// A key returned is saved on the server. A key not returned isn't, if the refresh returned
    /// them all, or if the server gave the row a date no older than the oldest key returned: past
    /// 1,000 keys, an older one may just be past the end. So may a row with only a tap's date,
    /// since saving a post already saved keeps its old date on the server, so those are left
    /// alone. A row that agreed with the server, with no tap since, follows it; a pending one
    /// stays pending. So does one in doubt, even if it reads as agreeing: its request may have
    /// landed since, so what the server last said is out of date, and the user's last tap stands.
    /// A refresh answers a request in doubt, so its tap is done. Rows the server and the user agree
    /// are unsaved, with no tap to send, are removed.
    pub fn apply_keys(
        executor: &impl QueryExecutor,
        account: RowId,
        keys: &[DbReaderSavedKey],
        returned: u64,
        total: u64,
    ) -> Result<(), SqliteDbError> {
        let complete = returned >= total;
        let mut seen = Vec::new();
        for entry in keys {
            let text = reader_saved_key_text(&entry.key);
            // An empty URL isn't one.
            let url = entry.url.as_ref().filter(|url| !url.is_empty());
            // A row saved by the URL is the same save.
            if let Some(url) = url {
                let url_key = ReaderSavedKey::Url { url: url.clone() };
                let url_text = reader_saved_key_text(&url_key);
                if url_text != text {
                    for row in Self::select_where(executor, account, "s.key = ?2", &[&url_text])? {
                        Self::rekey(executor, account, &row, &entry.key)?;
                    }
                }
            }
            let existing = Self::select_where(executor, account, "s.key = ?2", &[&text])?;
            let row_id = match existing.into_iter().next() {
                Some(row) => {
                    // A tapped row is the user's newer word, so it doesn't follow the keys.
                    let agreed = !row.in_doubt && !row.tapped && row.server == Some(row.saved);
                    // Only a row this changes is written, so a refresh that changes nothing
                    // tells nobody the list changed. The date is filled in, not replaced: the
                    // list's pages give it too, and the two mustn't take turns changing it.
                    executor.execute(
                        "UPDATE reader_saved_posts SET
                            server = 1, in_doubt = 0, waiting_for_room = 0,
                            date_saved = CASE WHEN date_known THEN COALESCE(date_saved, ?2)
                                ELSE ?2 END,
                            date_known = 1,
                            tapped = CASE WHEN in_doubt THEN 0 ELSE tapped END,
                            saved = CASE WHEN ?3 THEN 1 ELSE saved END
                        WHERE rowid = ?1 AND (server IS NOT 1 OR in_doubt != 0 OR waiting_for_room != 0
                            OR date_saved IS NULL OR date_known = 0 OR (?3 AND saved != 1))",
                        params![row.row_id, reader_date_text(&entry.date_saved), agreed],
                    )?;
                    row.row_id
                }
                None => {
                    let row_id =
                        Self::insert(executor, account, &entry.key, true, Some(true), 0, false)?;
                    executor.execute(
                        "UPDATE reader_saved_posts SET date_saved = ?2, date_known = 1 WHERE rowid = ?1",
                        params![row_id, reader_date_text(&entry.date_saved)],
                    )?;
                    row_id
                }
            };
            if let Some(url) = url {
                Self::add_aliases(
                    executor,
                    row_id,
                    &entry.key,
                    &[ReaderSavedKey::Url { url: url.clone() }],
                )?;
            }
            seen.push(row_id);
        }

        let oldest = keys
            .iter()
            .map(|entry| reader_date_text(&entry.date_saved))
            .min();
        let seen_list = seen
            .iter()
            .map(|row| row.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        executor.execute(
            &format!(
                "UPDATE reader_saved_posts SET
                    saved = CASE WHEN in_doubt = 0 AND tapped = 0 AND server IS NOT NULL
                        AND server = saved THEN 0 ELSE saved END,
                    server = 0, in_doubt = 0,
                    tapped = CASE WHEN in_doubt THEN 0 ELSE tapped END
                WHERE account_id = ?1 AND rowid NOT IN ({seen_list})
                    AND (?2 OR date_saved IS NULL OR (date_known = 1 AND date_saved >= ?3))
                    AND (server IS NOT 0 OR in_doubt != 0)"
            ),
            params![account, complete, oldest],
        )?;
        // Past the end of the keys, a row in doubt they can't decide is sent again: saving and
        // unsaving are idempotent, and nothing else would ever decide it.
        executor.execute(
            &format!(
                "UPDATE reader_saved_posts SET in_doubt = 0
                WHERE account_id = ?1 AND in_doubt = 1 AND rowid NOT IN ({seen_list})"
            ),
            [account],
        )?;
        executor.execute(
            "DELETE FROM reader_saved_posts
            WHERE account_id = ?1 AND saved = 0 AND server = 0 AND tapped = 0",
            [account],
        )?;
        executor.execute(
            "INSERT INTO reader_saved_state (account_id, keys_refreshed, keys_complete, total)
            VALUES (?1, 1, ?2, ?3)
            ON CONFLICT(account_id) DO UPDATE SET
                keys_refreshed = 1, keys_complete = excluded.keys_complete, total = excluded.total
            WHERE keys_refreshed != 1 OR keys_complete != excluded.keys_complete
                OR total IS NOT excluded.total",
            params![account, complete, total],
        )?;
        Ok(())
    }

    /// Records that the saved list sent a post: it's saved, and shows in the list. `keys` are the
    /// keys it may be saved under, `main` first. A row that's already there keeps what the user
    /// wants and what the server said.
    pub fn apply_listed(
        executor: &impl QueryExecutor,
        account: RowId,
        main: &ReaderSavedKey,
        keys: &[ReaderSavedKey],
        post: RowId,
        date_saved: Option<&WpGmtDateTime>,
    ) -> Result<(), SqliteDbError> {
        let rows = Self::select_rows_for_keys(executor, account, keys)?;
        let row_id = match rows.first() {
            Some(row) => row.row_id,
            None => Self::insert(executor, account, main, true, Some(true), 0, true)?,
        };
        executor.execute(
            "UPDATE reader_saved_posts SET
                listed = 1, post_id = ?2, date_saved = COALESCE(?3, date_saved),
                date_known = CASE WHEN ?3 IS NULL THEN date_known ELSE 1 END
            WHERE rowid = ?1 AND (listed != 1 OR post_id IS NOT ?2
                OR (?3 IS NOT NULL AND (date_saved IS NOT ?3 OR date_known = 0)))",
            params![row_id, post, date_saved.map(reader_date_text)],
        )?;
        Self::add_aliases(executor, row_id, main, keys)
    }

    /// Adds an imported save, as a save would: a post already saved is left as it is, and one
    /// unsaved, or with an unsave pending, is saved again. Returns whether it changed anything.
    pub fn import_save(
        executor: &impl QueryExecutor,
        account: RowId,
        key: &ReaderSavedKey,
        post: Option<RowId>,
        date_saved: Option<&WpGmtDateTime>,
    ) -> Result<bool, SqliteDbError> {
        let rows = Self::select_rows_for_keys(executor, account, std::slice::from_ref(key))?;
        if rows.iter().any(|row| row.saved) {
            return Ok(false);
        }
        let seq = Self::next_seq(executor, account)?;
        let row_id = match rows.first() {
            Some(row) => {
                executor.execute(
                    "UPDATE reader_saved_posts SET saved = 1, seq = ?2, listed = 1, date_known = 0
                    WHERE rowid = ?1",
                    params![row.row_id, seq],
                )?;
                row.row_id
            }
            None => Self::insert(executor, account, key, true, None, seq, true)?,
        };
        executor.execute(
            "UPDATE reader_saved_posts SET imported = 1, post_id = COALESCE(post_id, ?2),
                date_saved = COALESCE(?3, date_saved)
            WHERE rowid = ?1",
            params![row_id, post, date_saved.map(reader_date_text)],
        )?;
        Ok(true)
    }

    /// The newest total the server gave, moved by what's pending: `None` before any.
    pub fn saved_count(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<Option<u64>, SqliteDbError> {
        let Some(total) = Self::select_state(executor, account)?.total else {
            return Ok(None);
        };
        let mut statement = executor.prepare(
            "SELECT COALESCE(SUM(CASE
                WHEN server IS NULL THEN saved
                ELSE saved - server END), 0)
            FROM reader_saved_posts WHERE account_id = ?1 AND waiting_for_room = 0",
        )?;
        let delta: i64 = statement.query_row([account], |row| row.get(0))?;
        Ok(Some((total as i64 + delta).max(0) as u64))
    }

    /// The saved list: rows saved and listed, newest first, with their posts.
    pub fn select_list(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<Vec<DbReaderSavedListItem>, SqliteDbError> {
        let sql = format!(
            "SELECT {ROW_COLUMNS}, {POST_COLUMNS} FROM reader_saved_posts s
            LEFT JOIN reader_posts p ON p.rowid = s.post_id
            WHERE s.account_id = ?1 AND s.saved = 1 AND s.listed = 1
            ORDER BY s.date_saved IS NULL, s.date_saved DESC, s.seq DESC, s.rowid DESC"
        );
        let mut statement = executor.prepare(&sql)?;
        let rows = statement.query_map([account], |row| {
            Ok((|| -> Result<DbReaderSavedListItem, SqliteDbError> {
                let saved = saved_row_from_row(row)?;
                let post = match saved.post_row_id {
                    Some(_) => Some(post_from_row(row, ROW_COLUMN_COUNT)?.1),
                    None => None,
                };
                Ok(DbReaderSavedListItem { row: saved, post })
            })())
        })?;
        collect_rows(rows.map(|row| row.map_err(SqliteDbError::from).and_then(|row| row)))
    }
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
