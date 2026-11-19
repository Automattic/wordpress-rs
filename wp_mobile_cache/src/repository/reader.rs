//! Reader posts, streams, and saved posts state. Everything here belongs to a WordPress.com account
//! (see [`super::wordpress_com_accounts`]) and goes with it.

use crate::{
    RowId, SqliteDbError,
    db_types::reader::{
        DbReaderPageEntry, DbReaderPageItem, DbReaderPageOutcome, DbReaderSavedKey,
        DbReaderSavedListItem, DbReaderSavedRow, DbReaderSavedState, DbReaderStream,
        DbReaderStreamCard, DbReaderStreamItem, DbReaderUnreadablePost, ReaderPostRecord,
        reader_card_type_from_number, reader_card_type_number, reader_date_from_text,
        reader_date_text, reader_saved_key_from_text, reader_saved_key_text,
    },
    list_metadata::ListState,
    repository::QueryExecutor,
    repository::reader_offline::{OFFLINE_COLUMNS, offline_from_row},
};
use rusqlite::{OptionalExtension, Row, params, types::Value};
use wp_api::date::WpGmtDateTime;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_posts::{ReaderCrossPost, ReaderStreamOrder};
use wp_api::wp_com::reader_saved::{
    ReaderFeedId, ReaderFeedItemId, ReaderPostSaveIdentity, ReaderSavedKey, ReaderSavedOrderBy,
};

/// The `reader_posts` columns a [`ReaderPostRecord`] is read from.
const POST_FIELDS: &[&str] = &[
    "rowid",
    "identity_key",
    "post_id",
    "site_id",
    "feed_id",
    "feed_item_id",
    "is_external",
    "url",
    "global_id",
    "short_url",
    "title",
    "excerpt",
    "content",
    "use_excerpt",
    "date",
    "modified",
    "author_id",
    "author_name",
    "author_avatar_url",
    "author_url",
    "featured_image",
    "display_image_url",
    "editorial_image_url",
    "site_name",
    "site_url",
    "feed_url",
    "site_icon_url",
    "site_locale",
    "site_is_private",
    "site_is_atomic",
    "is_jetpack",
    "organization_id",
    "tags",
    "xpost_site_id",
    "xpost_post_id",
    "xpost_post_url",
    "xpost_comment_url",
    "like_count",
    "i_like",
    "likes_enabled",
    "is_following",
    "comment_count",
    "comments_open",
    "can_subscribe_comments",
    "is_subscribed_comments",
    "receives_comment_notifications",
    "is_seen",
    "word_count",
    "summary",
    "reading_time_minutes",
    "card_type",
];

/// What to read of each post.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PostContent {
    Included,
    /// The content is read as `NULL`. It's most of a post's size, and a list of posts doesn't show
    /// it.
    LeftOut,
}

/// [`POST_FIELDS`] as a column list for a `SELECT` from `reader_posts p`. Each column is named
/// `p_<field>`, which is what [`post_from_row`] reads it by, so the list can sit anywhere among a
/// query's other columns.
fn post_columns(content: PostContent) -> String {
    POST_FIELDS
        .iter()
        .map(|field| match (*field, content) {
            ("content", PostContent::LeftOut) => "NULL AS p_content".to_string(),
            _ => format!("p.{field} AS p_{field}"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Reads a post from the columns of [`post_columns`].
fn post_from_row(row: &Row) -> Result<(RowId, ReaderPostRecord), SqliteDbError> {
    let optional_date = |name: &str| -> Result<_, SqliteDbError> {
        row.get::<_, Option<String>>(name)?
            .map(|text| reader_date_from_text(&text))
            .transpose()
    };
    let identity_key: String = row.get("p_identity_key")?;
    let editorial_image_url: Option<String> = row.get("p_editorial_image_url")?;
    let cross_post = ReaderCrossPost {
        site_id: row
            .get::<_, Option<u64>>("p_xpost_site_id")?
            .map(WpComSiteId),
        post_id: row.get("p_xpost_post_id")?,
        post_url: row.get("p_xpost_post_url")?,
        comment_url: row.get("p_xpost_comment_url")?,
    };
    let is_cross_post = cross_post.site_id.is_some()
        || cross_post.post_id.is_some()
        || cross_post.post_url.is_some()
        || cross_post.comment_url.is_some();
    let record = ReaderPostRecord {
        key: reader_saved_key_from_text(&identity_key)?,
        identity: ReaderPostSaveIdentity {
            id: row.get("p_post_id")?,
            site_id: row.get::<_, Option<u64>>("p_site_id")?.map(WpComSiteId),
            feed_id: row.get::<_, Option<u64>>("p_feed_id")?.map(ReaderFeedId),
            feed_item_id: row
                .get::<_, Option<u64>>("p_feed_item_id")?
                .map(ReaderFeedItemId),
            is_external: row.get("p_is_external")?,
            url: row.get("p_url")?,
        },
        global_id: row.get("p_global_id")?,
        title: row.get("p_title")?,
        excerpt: row.get("p_excerpt")?,
        content: row.get("p_content")?,
        date: optional_date("p_date")?,
        modified: optional_date("p_modified")?,
        author_name: row.get("p_author_name")?,
        author_avatar_url: row.get("p_author_avatar_url")?,
        author_url: row.get("p_author_url")?,
        featured_image: row.get("p_featured_image")?,
        site_name: row.get("p_site_name")?,
        site_url: row.get("p_site_url")?,
        feed_url: row.get("p_feed_url")?,
        like_count: row.get("p_like_count")?,
        i_like: row.get("p_i_like")?,
        is_following: row.get("p_is_following")?,
        site_icon_url: row.get("p_site_icon_url")?,
        tags: row
            .get::<_, Option<String>>("p_tags")?
            .map(|json| serde_json::from_str(&json))
            .transpose()
            .map_err(|error| {
                SqliteDbError::SqliteError(format!("Invalid Reader post tags: {error}"))
            })?,
        comment_count: row.get("p_comment_count")?,
        // An editor's pick is shown ahead of the stored image.
        display_image_url: editorial_image_url
            .clone()
            .or(row.get("p_display_image_url")?),
        editorial_image_url,
        short_url: row.get("p_short_url")?,
        use_excerpt: row.get("p_use_excerpt")?,
        author_id: row.get("p_author_id")?,
        site_locale: row.get("p_site_locale")?,
        site_is_private: row.get("p_site_is_private")?,
        site_is_atomic: row.get("p_site_is_atomic")?,
        is_jetpack: row.get("p_is_jetpack")?,
        organization_id: row.get("p_organization_id")?,
        cross_post: is_cross_post.then_some(cross_post),
        likes_enabled: row.get("p_likes_enabled")?,
        comments_open: row.get("p_comments_open")?,
        can_subscribe_comments: row.get("p_can_subscribe_comments")?,
        is_subscribed_comments: row.get("p_is_subscribed_comments")?,
        receives_comment_notifications: row.get("p_receives_comment_notifications")?,
        is_seen: row.get("p_is_seen")?,
        word_count: row.get("p_word_count")?,
        summary: row.get("p_summary")?,
        reading_time_minutes: row.get("p_reading_time_minutes")?,
        card_type: row
            .get::<_, Option<i64>>("p_card_type")?
            .map(reader_card_type_from_number),
    };
    Ok((row.get("p_rowid")?, record))
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

    /// Like [`Self::fill_missing`], into the stored post `row`, whatever key it's stored under. For
    /// details of a post found under another key, such as a Jetpack post's feed item copy that's
    /// stored as its WordPress.com post, so they don't make a second row for the same post.
    pub fn fill_missing_at(
        executor: &impl QueryExecutor,
        account: RowId,
        row: RowId,
        post: &ReaderPostRecord,
    ) -> Result<RowId, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT identity_key FROM reader_posts WHERE rowid = ?1 AND account_id = ?2",
        )?;
        let identity_key: String = statement.query_row(params![row, account], |row| row.get(0))?;
        let post = ReaderPostRecord {
            key: reader_saved_key_from_text(&identity_key)?,
            ..post.clone()
        };
        Self::store(executor, account, &post, Prefer::Stored)
    }

    fn store(
        executor: &impl QueryExecutor,
        account: RowId,
        post: &ReaderPostRecord,
        prefer: Prefer,
    ) -> Result<RowId, SqliteDbError> {
        let identity_key = reader_saved_key_text(&post.key);
        let tags = post
            .tags
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|error| {
                SqliteDbError::SqliteError(format!("Couldn't store Reader post tags: {error}"))
            })?;
        let text = |value: &Option<String>| Value::from(value.clone());
        let date =
            |value: &Option<WpGmtDateTime>| Value::from(value.as_ref().map(reader_date_text));
        let number = |value: Option<u64>| Value::from(value.map(|value| value as i64));
        let flag = |value: Option<bool>| Value::from(value);
        let cross_post = post.cross_post.as_ref();
        // The columns a source may leave out, with the post's value for each.
        let optional: Vec<(&str, Value)> = vec![
            ("site_id", number(post.identity.site_id.map(|id| id.0))),
            ("feed_id", number(post.identity.feed_id.map(|id| id.0))),
            (
                "feed_item_id",
                number(post.identity.feed_item_id.map(|id| id.0)),
            ),
            ("url", text(&post.identity.url)),
            ("global_id", text(&post.global_id)),
            ("short_url", text(&post.short_url)),
            ("title", text(&post.title)),
            ("excerpt", text(&post.excerpt)),
            ("content", text(&post.content)),
            ("use_excerpt", flag(post.use_excerpt)),
            ("date", date(&post.date)),
            ("modified", date(&post.modified)),
            ("author_id", number(post.author_id)),
            ("author_name", text(&post.author_name)),
            ("author_avatar_url", text(&post.author_avatar_url)),
            ("author_url", text(&post.author_url)),
            ("featured_image", text(&post.featured_image)),
            ("display_image_url", text(&post.display_image_url)),
            ("editorial_image_url", text(&post.editorial_image_url)),
            ("site_name", text(&post.site_name)),
            ("site_url", text(&post.site_url)),
            ("feed_url", text(&post.feed_url)),
            ("site_icon_url", text(&post.site_icon_url)),
            ("site_locale", text(&post.site_locale)),
            ("site_is_private", flag(post.site_is_private)),
            ("site_is_atomic", flag(post.site_is_atomic)),
            ("is_jetpack", flag(post.is_jetpack)),
            ("organization_id", number(post.organization_id)),
            ("tags", Value::from(tags)),
            (
                "xpost_site_id",
                number(cross_post.and_then(|post| post.site_id).map(|id| id.0)),
            ),
            (
                "xpost_post_id",
                number(cross_post.and_then(|post| post.post_id)),
            ),
            (
                "xpost_post_url",
                Value::from(cross_post.and_then(|post| post.post_url.clone())),
            ),
            (
                "xpost_comment_url",
                Value::from(cross_post.and_then(|post| post.comment_url.clone())),
            ),
            ("like_count", number(post.like_count)),
            ("i_like", flag(post.i_like)),
            ("likes_enabled", flag(post.likes_enabled)),
            ("is_following", flag(post.is_following)),
            ("comment_count", number(post.comment_count)),
            ("comments_open", flag(post.comments_open)),
            ("can_subscribe_comments", flag(post.can_subscribe_comments)),
            ("is_subscribed_comments", flag(post.is_subscribed_comments)),
            (
                "receives_comment_notifications",
                flag(post.receives_comment_notifications),
            ),
            ("is_seen", flag(post.is_seen)),
            ("word_count", number(post.word_count)),
            ("summary", text(&post.summary)),
            (
                "reading_time_minutes",
                number(post.reading_time_minutes.map(u64::from)),
            ),
            (
                "card_type",
                Value::from(post.card_type.map(reader_card_type_number)),
            ),
        ];

        let mut updates: Vec<String> = Vec::new();
        if prefer == Prefer::Incoming {
            updates.push("post_id = excluded.post_id".to_string());
            updates.push("is_external = excluded.is_external".to_string());
        }
        for (column, _) in &optional {
            updates.push(match prefer {
                Prefer::Incoming => format!("{column} = COALESCE(excluded.{column}, {column})"),
                Prefer::Stored => format!("{column} = COALESCE({column}, excluded.{column})"),
            });
        }
        if prefer == Prefer::Incoming {
            updates.push("fetched_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')".to_string());
        }
        let columns: Vec<&str> = optional.iter().map(|(column, _)| *column).collect();
        let placeholders: Vec<String> = (0..optional.len())
            .map(|index| format!("?{}", index + 5))
            .collect();
        let sql = format!(
            "INSERT INTO reader_posts (account_id, identity_key, post_id, is_external, {})
            VALUES (?1, ?2, ?3, ?4, {})
            ON CONFLICT(account_id, identity_key) DO UPDATE SET {}",
            columns.join(", "),
            placeholders.join(", "),
            updates.join(", ")
        );
        let mut values: Vec<Value> = vec![
            Value::from(account.0),
            Value::from(identity_key.clone()),
            Value::from(post.identity.id as i64),
            Value::from(post.identity.is_external),
        ];
        values.extend(optional.into_iter().map(|(_, value)| value));
        executor.execute(&sql, rusqlite::params_from_iter(values))?;
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
            "SELECT {} FROM reader_posts p WHERE p.account_id = ?1 AND p.identity_key = ?2",
            post_columns(PostContent::Included)
        );
        let mut statement = executor.prepare(&sql)?;
        let exact = statement
            .query_row(params![account, reader_saved_key_text(key)], |row| {
                Ok(post_from_row(row))
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
            "SELECT {} FROM reader_posts p
            WHERE p.account_id = ?1 AND p.feed_item_id = ?2
            ORDER BY p.rowid LIMIT 1",
            post_columns(PostContent::Included)
        );
        let mut statement = executor.prepare(&sql)?;
        statement
            .query_row(params![account, feed_item_id.0], |row| {
                Ok(post_from_row(row))
            })
            .optional()?
            .transpose()
    }

    /// The post in a row, with its content.
    pub fn select_by_row_id(
        executor: &impl QueryExecutor,
        post: RowId,
    ) -> Result<Option<ReaderPostRecord>, SqliteDbError> {
        let sql = format!(
            "SELECT {} FROM reader_posts p WHERE p.rowid = ?1",
            post_columns(PostContent::Included)
        );
        let mut statement = executor.prepare(&sql)?;
        Ok(statement
            .query_row([post], |row| Ok(post_from_row(row)))
            .optional()?
            .transpose()?
            .map(|(_, post)| post))
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

    /// The stream's posts, newest first, without posts from a site or by a user the reader blocked.
    pub fn select_items(
        executor: &impl QueryExecutor,
        stream: RowId,
    ) -> Result<Vec<DbReaderStreamItem>, SqliteDbError> {
        Self::select_items_with(executor, stream, PostContent::Included)
    }

    /// The stream's posts, newest first, each without its content. For a list of posts, which
    /// doesn't show it: a post's content is most of its size.
    pub fn select_items_without_content(
        executor: &impl QueryExecutor,
        stream: RowId,
    ) -> Result<Vec<DbReaderStreamItem>, SqliteDbError> {
        Self::select_items_with(executor, stream, PostContent::LeftOut)
    }

    fn select_items_with(
        executor: &impl QueryExecutor,
        stream: RowId,
        content: PostContent,
    ) -> Result<Vec<DbReaderStreamItem>, SqliteDbError> {
        let sql = format!(
            "SELECT {}, i.sort_key AS i_sort_key, i.gap_before AS i_gap_before
            FROM reader_stream_items i
            JOIN reader_posts p ON p.rowid = i.post_id
            WHERE i.stream_id = ?1
                AND NOT EXISTS (
                    SELECT 1 FROM reader_blocked_sites b
                    WHERE b.account_id = p.account_id AND b.site_id = p.site_id)
                AND NOT EXISTS (
                    SELECT 1 FROM reader_blocked_users u
                    WHERE u.account_id = p.account_id AND u.user_id = p.author_id)
            ORDER BY i.sort_key DESC, i.rowid ASC",
            post_columns(content)
        );
        let mut statement = executor.prepare(&sql)?;
        let rows = statement.query_map([stream], |row| {
            Ok(post_from_row(row).and_then(|(post_row_id, post)| {
                Ok(DbReaderStreamItem {
                    post,
                    post_row_id,
                    sort_key: row.get("i_sort_key")?,
                    gap_before: row.get("i_gap_before")?,
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

    /// How many posts the stream has.
    pub fn count_items(executor: &impl QueryExecutor, stream: RowId) -> Result<u64, SqliteDbError> {
        let mut statement =
            executor.prepare("SELECT COUNT(*) FROM reader_stream_items WHERE stream_id = ?1")?;
        Ok(statement.query_row([stream], |row| row.get(0))?)
    }

    fn has_item(
        executor: &impl QueryExecutor,
        stream: RowId,
        post: RowId,
    ) -> Result<bool, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT EXISTS(SELECT 1 FROM reader_stream_items WHERE stream_id = ?1 AND post_id = ?2)",
        )?;
        Ok(statement.query_row(params![stream, post], |row| row.get(0))?)
    }

    /// Post rows as a JSON array, for `json_each` in a `NOT IN`.
    fn rows_json<'a>(rows: impl Iterator<Item = &'a RowId>) -> String {
        let ids: Vec<String> = rows.map(|row| row.0.to_string()).collect();
        format!("[{}]", ids.join(","))
    }

    /// Adds a page to a stream ordered by date, and makes the stream agree with it.
    ///
    /// The page is the server's posts older than `before` (`None` for the newest), down to its
    /// oldest post. So between those two ends the page is the whole truth:
    ///
    /// - A post the stream has there that the page doesn't was deleted, or left the stream, and is
    ///   removed. An empty page means there is nothing older than `before`. `keep` are posts to
    ///   leave alone, such as ones the page had and couldn't read.
    /// - Each of the page's posts follows the one above it, so none has a gap before it.
    /// - If the stream had none of the page's posts, nothing says the page reaches the older posts
    ///   the stream has. The newest of those is marked as having a gap before it.
    ///
    /// A post dated the same as the page's oldest is left as it is: the page may have ended
    /// between two posts with one date. Call it inside a transaction.
    pub fn apply_dated_page(
        executor: &impl QueryExecutor,
        stream: RowId,
        before: Option<&str>,
        items: &[DbReaderPageItem],
        keep: &[RowId],
    ) -> Result<DbReaderPageOutcome, SqliteDbError> {
        let mut outcome = DbReaderPageOutcome::default();
        for item in items {
            if Self::has_item(executor, stream, item.post)? {
                outcome.overlapped = true;
            } else {
                outcome.added += 1;
            }
            executor.execute(
                "INSERT INTO reader_stream_items (stream_id, post_id, sort_key, railcar, gap_before)
                VALUES (?1, ?2, ?3, ?4, 0)
                ON CONFLICT(stream_id, post_id) DO UPDATE SET
                    sort_key = excluded.sort_key,
                    railcar = COALESCE(excluded.railcar, railcar),
                    gap_before = 0",
                params![stream, item.post, item.sort_key, item.railcar],
            )?;
        }

        let oldest = items.iter().map(|item| item.sort_key.as_str()).min();
        let page = Self::rows_json(items.iter().map(|item| &item.post));
        let protected = Self::rows_json(items.iter().map(|item| &item.post).chain(keep));
        outcome.removed = executor.execute(
            "DELETE FROM reader_stream_items
            WHERE stream_id = ?1
                AND (?2 IS NULL OR sort_key < ?2)
                AND (?3 IS NULL OR sort_key > ?3)
                AND post_id NOT IN (SELECT value FROM json_each(?4))",
            params![stream, before, oldest, protected],
        )? as u64;

        if let Some(oldest) = oldest
            && !outcome.overlapped
        {
            executor.execute(
                "UPDATE reader_stream_items SET gap_before = 1 WHERE rowid = (
                    SELECT rowid FROM reader_stream_items
                    WHERE stream_id = ?1 AND sort_key <= ?2
                        AND post_id NOT IN (SELECT value FROM json_each(?3))
                    ORDER BY sort_key DESC, rowid ASC LIMIT 1)",
                params![stream, oldest, page],
            )?;
        }
        Ok(outcome)
    }

    /// Where a post sorts in a stream kept in the order the server sent: the first post sorts
    /// highest, as the newest date does in a stream ordered by date.
    fn position_sort_key(position: u64) -> String {
        format!("{:010}", 9_999_999_999u64.saturating_sub(position))
    }

    /// Makes a stream kept in the order the server sent hold exactly `entries`, in that order. For
    /// the first page of such a stream: its later pages follow from it, so what the stream had
    /// past it can't be kept. Call it inside a transaction.
    pub fn replace_entries(
        executor: &impl QueryExecutor,
        stream: RowId,
        entries: &[DbReaderPageEntry],
    ) -> Result<DbReaderPageOutcome, SqliteDbError> {
        let posts = entries.iter().filter_map(|entry| match entry {
            DbReaderPageEntry::Post(item) => Some(&item.post),
            DbReaderPageEntry::Card(_) => None,
        });
        let mut outcome = DbReaderPageOutcome {
            removed: executor.execute(
                "DELETE FROM reader_stream_items
                WHERE stream_id = ?1 AND post_id NOT IN (SELECT value FROM json_each(?2))",
                params![stream, Self::rows_json(posts)],
            )? as u64,
            ..Default::default()
        };
        executor.execute(
            "DELETE FROM reader_stream_cards WHERE stream_id = ?1",
            [stream],
        )?;
        Self::place_entries(executor, stream, entries, 0, &mut outcome)?;
        Ok(outcome)
    }

    /// Adds a later page to a stream kept in the order the server sent, after what it has. A post
    /// the stream already has stays where it is. Call it inside a transaction.
    pub fn append_entries(
        executor: &impl QueryExecutor,
        stream: RowId,
        entries: &[DbReaderPageEntry],
    ) -> Result<DbReaderPageOutcome, SqliteDbError> {
        // One past the lowest place taken, by a post or a card. Places aren't reused, so this is
        // past everything.
        let mut statement = executor.prepare(
            "SELECT MIN(sort_key) FROM (
                SELECT sort_key FROM reader_stream_items WHERE stream_id = ?1
                UNION ALL SELECT sort_key FROM reader_stream_cards WHERE stream_id = ?1)",
        )?;
        let lowest: Option<String> = statement.query_row([stream], |row| row.get(0))?;
        let position = match lowest {
            Some(key) => {
                let taken: u64 = key.parse().map_err(|_| {
                    SqliteDbError::SqliteError(format!("Invalid Reader stream position: {key}"))
                })?;
                9_999_999_999u64.saturating_sub(taken) + 1
            }
            None => 0,
        };
        let mut outcome = DbReaderPageOutcome::default();
        Self::place_entries(executor, stream, entries, position, &mut outcome)?;
        Ok(outcome)
    }

    /// Gives each entry the next place from `position`. A post the stream has keeps its place on a
    /// later page, and takes its new one on a first page, where the places start again.
    fn place_entries(
        executor: &impl QueryExecutor,
        stream: RowId,
        entries: &[DbReaderPageEntry],
        mut position: u64,
        outcome: &mut DbReaderPageOutcome,
    ) -> Result<(), SqliteDbError> {
        let first_page = position == 0;
        let mut placed: Vec<RowId> = Vec::new();
        for entry in entries {
            match entry {
                DbReaderPageEntry::Post(item) => {
                    // A post the server sent twice keeps its first place.
                    if placed.contains(&item.post) {
                        continue;
                    }
                    placed.push(item.post);
                    if Self::has_item(executor, stream, item.post)? {
                        outcome.overlapped = true;
                        if !first_page {
                            continue;
                        }
                    } else {
                        outcome.added += 1;
                    }
                    executor.execute(
                        "INSERT INTO reader_stream_items
                            (stream_id, post_id, sort_key, railcar, gap_before)
                        VALUES (?1, ?2, ?3, ?4, 0)
                        ON CONFLICT(stream_id, post_id) DO UPDATE SET
                            sort_key = excluded.sort_key,
                            railcar = COALESCE(excluded.railcar, railcar),
                            gap_before = 0",
                        params![
                            stream,
                            item.post,
                            Self::position_sort_key(position),
                            item.railcar
                        ],
                    )?;
                }
                DbReaderPageEntry::Card(card) => {
                    executor.execute(
                        "INSERT INTO reader_stream_cards (stream_id, sort_key, card)
                        VALUES (?1, ?2, ?3)",
                        params![stream, Self::position_sort_key(position), card],
                    )?;
                }
            }
            position += 1;
        }
        Ok(())
    }

    /// The stream's cards that aren't posts, in the stream's order, each with the post it comes
    /// after among the posts [`Self::select_items`] returns.
    pub fn select_cards(
        executor: &impl QueryExecutor,
        stream: RowId,
    ) -> Result<Vec<DbReaderStreamCard>, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT c.card, (
                SELECT p.identity_key FROM reader_stream_items i
                JOIN reader_posts p ON p.rowid = i.post_id
                WHERE i.stream_id = c.stream_id AND i.sort_key > c.sort_key
                    AND NOT EXISTS (
                        SELECT 1 FROM reader_blocked_sites b
                        WHERE b.account_id = p.account_id AND b.site_id = p.site_id)
                    AND NOT EXISTS (
                        SELECT 1 FROM reader_blocked_users u
                        WHERE u.account_id = p.account_id AND u.user_id = p.author_id)
                ORDER BY i.sort_key ASC, i.rowid DESC LIMIT 1)
            FROM reader_stream_cards c
            WHERE c.stream_id = ?1
            ORDER BY c.sort_key DESC, c.rowid ASC",
        )?;
        let rows = statement.query_map([stream], |row| {
            Ok((|| -> Result<DbReaderStreamCard, SqliteDbError> {
                Ok(DbReaderStreamCard {
                    card: row.get(0)?,
                    after_post: row
                        .get::<_, Option<String>>(1)?
                        .map(|key| reader_saved_key_from_text(&key))
                        .transpose()?,
                })
            })())
        })?;
        collect_rows(rows.map(|row| row.map_err(SqliteDbError::from).and_then(|row| row)))
    }

    /// The sort key of the stream's last post: the oldest, in a stream ordered by date.
    pub fn select_last_sort_key(
        executor: &impl QueryExecutor,
        stream: RowId,
    ) -> Result<Option<String>, SqliteDbError> {
        let mut statement = executor
            .prepare("SELECT MIN(sort_key) FROM reader_stream_items WHERE stream_id = ?1")?;
        Ok(statement.query_row([stream], |row| row.get(0))?)
    }

    /// A post's place in the stream, if it's in it: its sort key, and whether it has a gap before
    /// it.
    pub fn select_item_place(
        executor: &impl QueryExecutor,
        stream: RowId,
        post: RowId,
    ) -> Result<Option<(String, bool)>, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT sort_key, gap_before FROM reader_stream_items
            WHERE stream_id = ?1 AND post_id = ?2",
        )?;
        Ok(statement
            .query_row(params![stream, post], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()?)
    }

    /// The sort key of the post shown just above a post, or `None` if it's the first.
    pub fn select_sort_key_above(
        executor: &impl QueryExecutor,
        stream: RowId,
        post: RowId,
    ) -> Result<Option<String>, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT above.sort_key FROM reader_stream_items item
            JOIN reader_stream_items above ON above.stream_id = item.stream_id
                AND (above.sort_key > item.sort_key
                    OR (above.sort_key = item.sort_key AND above.rowid < item.rowid))
            WHERE item.stream_id = ?1 AND item.post_id = ?2
            ORDER BY above.sort_key ASC, above.rowid DESC LIMIT 1",
        )?;
        Ok(statement
            .query_row(params![stream, post], |row| row.get(0))
            .optional()?)
    }

    /// Records that nothing is missing before a post any more.
    pub fn clear_gap(
        executor: &impl QueryExecutor,
        stream: RowId,
        post: RowId,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE reader_stream_items SET gap_before = 0 WHERE stream_id = ?1 AND post_id = ?2",
            params![stream, post],
        )?;
        Ok(())
    }

    /// Records that a page loaded, and that the stream is idle.
    ///
    /// `has_more` is whether the stream has posts past its last one, or `None` to leave what's
    /// recorded, which a page that didn't reach the end can't say. `next_page_handle` is kept for
    /// a stream that pages with one. `refreshed_at` is the time, when the page was the first. That
    /// also bumps the version: a later page that was asked for before it read the old handle.
    pub fn finish_page(
        executor: &impl QueryExecutor,
        stream: RowId,
        has_more: Option<bool>,
        next_page_handle: Option<&str>,
        refreshed_at: Option<&WpGmtDateTime>,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE reader_streams SET
                next_page_handle = ?2,
                has_more = COALESCE(?3, has_more),
                last_refreshed_at = COALESCE(?4, last_refreshed_at),
                version = version + CASE WHEN ?4 IS NULL THEN 0 ELSE 1 END,
                state = ?5,
                error_message = NULL
            WHERE rowid = ?1",
            params![
                stream,
                next_page_handle,
                has_more,
                refreshed_at.map(reader_date_text),
                ListState::Idle
            ],
        )?;
        Ok(())
    }

    /// Removes the stream's posts past its first `keep`. In a stream ordered by date those are the
    /// oldest, and they can be loaded again. Returns how many it removed.
    pub fn trim(
        executor: &impl QueryExecutor,
        stream: RowId,
        keep: u64,
    ) -> Result<u64, SqliteDbError> {
        let removed = executor.execute(
            "DELETE FROM reader_stream_items WHERE rowid IN (
                SELECT rowid FROM reader_stream_items WHERE stream_id = ?1
                ORDER BY sort_key DESC, rowid ASC LIMIT -1 OFFSET ?2)",
            params![stream, keep],
        )?;
        Ok(removed as u64)
    }

    /// Deletes the account's streams last refreshed before `before`, with their posts' places in
    /// them. `except` is a stream to keep whatever its age, such as the saved list, along with
    /// the streams whose key is `except`, a colon, and more: the saved list in another order.
    pub fn delete_stale(
        executor: &impl QueryExecutor,
        account: RowId,
        before: &WpGmtDateTime,
        except: &str,
    ) -> Result<u64, SqliteDbError> {
        let removed = executor.execute(
            "DELETE FROM reader_streams
            WHERE account_id = ?1 AND key != ?3 AND key NOT LIKE ?3 || ':%'
                AND state NOT IN (?4, ?5)
                AND (last_refreshed_at IS NULL OR last_refreshed_at < ?2)",
            params![
                account,
                reader_date_text(before),
                except,
                ListState::FetchingFirstPage,
                ListState::FetchingNextPage
            ],
        )?;
        Ok(removed as u64)
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

impl ReaderPostRepository {
    /// Deletes the account's posts that nothing shows any more: in no stream, not saved, and not
    /// stored since `fetched_before`. The last part keeps a post that was just fetched on its own,
    /// such as one opened from a link. Returns how many it deleted.
    pub fn delete_unused(
        executor: &impl QueryExecutor,
        account: RowId,
        fetched_before: &WpGmtDateTime,
    ) -> Result<u64, SqliteDbError> {
        let removed = executor.execute(
            "DELETE FROM reader_posts
            WHERE account_id = ?1 AND fetched_at < ?2
                AND rowid NOT IN (SELECT post_id FROM reader_stream_items)
                AND rowid NOT IN (
                    SELECT post_id FROM reader_saved_posts WHERE post_id IS NOT NULL)",
            params![account, reader_date_text(fetched_before)],
        )?;
        Ok(removed as u64)
    }
}

/// Something about a post that the user turns on or off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReaderPostFlag {
    IsSeen,
    ILike,
    IsFollowing,
    IsSubscribedComments,
    ReceivesCommentNotifications,
}

impl ReaderPostFlag {
    fn column(&self) -> &'static str {
        match self {
            Self::IsSeen => "is_seen",
            Self::ILike => "i_like",
            Self::IsFollowing => "is_following",
            Self::IsSubscribedComments => "is_subscribed_comments",
            Self::ReceivesCommentNotifications => "receives_comment_notifications",
        }
    }
}

impl ReaderPostRepository {
    /// A flag of a stored post, or `None` if the post has no value for it.
    pub fn select_flag(
        executor: &impl QueryExecutor,
        post: RowId,
        flag: ReaderPostFlag,
    ) -> Result<Option<bool>, SqliteDbError> {
        let sql = format!(
            "SELECT {} FROM reader_posts WHERE rowid = ?1",
            flag.column()
        );
        let mut statement = executor.prepare(&sql)?;
        Ok(statement
            .query_row([post], |row| row.get::<_, Option<bool>>(0))
            .optional()?
            .flatten())
    }

    /// Sets a flag of a stored post.
    pub fn set_flag(
        executor: &impl QueryExecutor,
        post: RowId,
        flag: ReaderPostFlag,
        value: bool,
    ) -> Result<(), SqliteDbError> {
        let sql = format!(
            "UPDATE reader_posts SET {} = ?2 WHERE rowid = ?1",
            flag.column()
        );
        executor.execute(&sql, params![post, value])?;
        Ok(())
    }

    /// Records whether the user likes a stored post, and moves its like count by one when that's a
    /// change: a like the cache didn't have adds one, and one it had taken away takes one off.
    pub fn set_liked(
        executor: &impl QueryExecutor,
        post: RowId,
        liked: bool,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE reader_posts SET
                like_count = CASE
                    WHEN COALESCE(i_like, 0) = ?2 THEN like_count
                    WHEN ?2 THEN COALESCE(like_count, 0) + 1
                    ELSE MAX(COALESCE(like_count, 0) - 1, 0) END,
                i_like = ?2
            WHERE rowid = ?1",
            params![post, liked],
        )?;
        Ok(())
    }

    /// Sets a stored post's like count to what the server said.
    pub fn set_like_count(
        executor: &impl QueryExecutor,
        post: RowId,
        like_count: u64,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE reader_posts SET like_count = ?2 WHERE rowid = ?1",
            params![post, like_count],
        )?;
        Ok(())
    }

    /// Records whether the user follows a site or a feed, on every stored post of either.
    pub fn set_following(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: Option<WpComSiteId>,
        feed_id: Option<ReaderFeedId>,
        following: bool,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE reader_posts SET is_following = ?4
            WHERE account_id = ?1 AND (
                (?2 IS NOT NULL AND site_id = ?2) OR (?3 IS NOT NULL AND feed_id = ?3))",
            params![
                account,
                site_id.map(|id| id.0),
                feed_id.map(|id| id.0),
                following
            ],
        )?;
        Ok(())
    }

    /// The name and URL of a site, from a post of it the cache has.
    pub fn select_site_name_and_url(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: WpComSiteId,
    ) -> Result<(Option<String>, Option<String>), SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT site_name, site_url FROM reader_posts
            WHERE account_id = ?1 AND site_id = ?2 AND (site_name IS NOT NULL OR site_url IS NOT NULL)
            ORDER BY rowid DESC LIMIT 1",
        )?;
        Ok(statement
            .query_row(params![account, site_id.0], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()?
            .unwrap_or((None, None)))
    }
}

/// Posts a stream's page had that couldn't be read. See the `reader_unreadable_posts` table.
pub struct ReaderUnreadablePostRepository;

impl ReaderUnreadablePostRepository {
    /// Records that a page of the stream had a post it couldn't read, or that it still can't. `now`
    /// is when, which is kept for a post that wasn't recorded yet.
    pub fn record(
        executor: &impl QueryExecutor,
        account: RowId,
        stream: RowId,
        key: &ReaderSavedKey,
        identity: &ReaderPostSaveIdentity,
        error: &str,
        now: &WpGmtDateTime,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "INSERT INTO reader_unreadable_posts (
                account_id, stream_id, identity_key, post_id, site_id, feed_id, feed_item_id,
                is_external, url, error, first_seen_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(stream_id, identity_key) DO UPDATE SET error = excluded.error",
            params![
                account,
                stream,
                reader_saved_key_text(key),
                identity.id,
                identity.site_id.map(|id| id.0),
                identity.feed_id.map(|id| id.0),
                identity.feed_item_id.map(|id| id.0),
                identity.is_external,
                identity.url,
                error,
                reader_date_text(now),
            ],
        )?;
        Ok(())
    }

    /// Forgets a post the stream could read after all. Returns whether it was recorded.
    pub fn resolve(
        executor: &impl QueryExecutor,
        stream: RowId,
        key: &ReaderSavedKey,
    ) -> Result<bool, SqliteDbError> {
        let removed = executor.execute(
            "DELETE FROM reader_unreadable_posts WHERE stream_id = ?1 AND identity_key = ?2",
            params![stream, reader_saved_key_text(key)],
        )?;
        Ok(removed > 0)
    }

    /// How many of the stream's posts couldn't be read.
    pub fn count(executor: &impl QueryExecutor, stream: RowId) -> Result<u64, SqliteDbError> {
        let mut statement = executor
            .prepare("SELECT COUNT(*) FROM reader_unreadable_posts WHERE stream_id = ?1")?;
        Ok(statement.query_row([stream], |row| row.get(0))?)
    }

    /// The stream's unreadable posts, oldest first.
    pub fn select(
        executor: &impl QueryExecutor,
        stream: RowId,
    ) -> Result<Vec<DbReaderUnreadablePost>, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT rowid, stream_id, post_id, site_id, feed_id, feed_item_id, is_external, url,
                error, attempts, last_attempt_at, first_seen_at
            FROM reader_unreadable_posts WHERE stream_id = ?1 ORDER BY rowid",
        )?;
        let rows = statement.query_map([stream], |row| {
            Ok((|| -> Result<DbReaderUnreadablePost, SqliteDbError> {
                Ok(DbReaderUnreadablePost {
                    row_id: row.get(0)?,
                    stream_row_id: row.get(1)?,
                    identity: ReaderPostSaveIdentity {
                        id: row.get(2)?,
                        site_id: row.get::<_, Option<u64>>(3)?.map(WpComSiteId),
                        feed_id: row.get::<_, Option<u64>>(4)?.map(ReaderFeedId),
                        feed_item_id: row.get::<_, Option<u64>>(5)?.map(ReaderFeedItemId),
                        is_external: row.get(6)?,
                        url: row.get(7)?,
                    },
                    error: row.get(8)?,
                    attempts: row.get(9)?,
                    first_seen_at: reader_date_from_text(&row.get::<_, String>(11)?)?,
                    last_attempt_at: row
                        .get::<_, Option<String>>(10)?
                        .map(|text| reader_date_from_text(&text))
                        .transpose()?,
                })
            })())
        })?;
        collect_rows(rows.map(|row| row.map_err(SqliteDbError::from).and_then(|row| row)))
    }

    /// Records that the post was asked for on its own, at `now`, and still couldn't be read.
    pub fn record_attempt(
        executor: &impl QueryExecutor,
        row: RowId,
        now: &WpGmtDateTime,
        error: &str,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE reader_unreadable_posts
            SET attempts = attempts + 1, last_attempt_at = ?2, error = ?3 WHERE rowid = ?1",
            params![row, reader_date_text(now), error],
        )?;
        Ok(())
    }

    pub fn delete(executor: &impl QueryExecutor, row: RowId) -> Result<(), SqliteDbError> {
        executor.execute(
            "DELETE FROM reader_unreadable_posts WHERE rowid = ?1",
            [row],
        )?;
        Ok(())
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

const ROW_COLUMNS: &str = "s.rowid, s.key, \
    (SELECT json_group_array(u.url_key) FROM reader_saved_post_urls u WHERE u.saved_post_id = s.rowid), \
    s.feed_item_key, s.saved, s.server, s.in_doubt, s.seq, s.date_saved, s.post_id, s.listed, \
    s.imported, s.waiting_for_room, s.tapped, s.date_known";

/// How many columns [`ROW_COLUMNS`] is.
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
        url_keys: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(2)?)
            .map_err(|error| SqliteDbError::SqliteError(error.to_string()))?
            .iter()
            .map(|text| reader_saved_key_from_text(text))
            .collect::<Result<_, _>>()?,
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
                "s.key = ?2 OR s.feed_item_key = ?2 OR s.rowid IN
                    (SELECT saved_post_id FROM reader_saved_post_urls WHERE url_key = ?2)",
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

    /// Records the other keys of a row: every URL the server stores as it, and the feed item of a
    /// Jetpack post, where it hasn't got one.
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
            match alias {
                // A post can have several URLs, so every one is kept.
                ReaderSavedKey::Url { .. } => {
                    executor.execute(
                        "INSERT OR IGNORE INTO reader_saved_post_urls (saved_post_id, url_key)
                        VALUES (?1, ?2)",
                        params![row_id, text],
                    )?;
                }
                ReaderSavedKey::FeedItem { .. } => {
                    executor.execute(
                        "UPDATE reader_saved_posts SET feed_item_key = ?2
                        WHERE rowid = ?1 AND feed_item_key IS NULL",
                        params![row_id, text],
                    )?;
                }
                ReaderSavedKey::Post { .. } => {}
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
    /// row gets one, so every key is sent: the keys may be out of date. That includes a save waiting
    /// for room, which another device may have saved since, and which stops waiting, as an unsave
    /// needs no room. Returns the rows to wait for.
    pub fn tap_unsave(
        executor: &impl QueryExecutor,
        account: RowId,
        keys: &[ReaderSavedKey],
    ) -> Result<Vec<RowId>, SqliteDbError> {
        let seq = Self::next_seq(executor, account)?;
        let rows = Self::select_rows_for_keys(executor, account, keys)?;
        let mut waiting = Vec::new();
        for row in &rows {
            executor.execute(
                "UPDATE reader_saved_posts SET saved = 0, seq = ?2, tapped = 1, waiting_for_room = 0
                WHERE rowid = ?1",
                params![row.row_id, seq],
            )?;
            waiting.push(row.row_id);
        }
        for key in keys {
            let text = reader_saved_key_text(key);
            let matched = rows.iter().any(|row| {
                // The server stores a URL as the post, but a feed item can be a separate, older
                // save of it, so the feed item is sent too.
                std::iter::once(&row.key)
                    .chain(&row.url_keys)
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
            &std::iter::once(from.key.clone())
                .chain(from.url_keys.iter().cloned())
                .chain(from.feed_item_key.clone())
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
    /// them all, or if the server gave the row a date newer than the oldest key returned: past
    /// 1,000 keys, an older one may just be past the end, and so may one with the same date, as
    /// the keys don't say how ties are ordered. So may a row with only a tap's date,
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
                    AND (?2 OR date_saved IS NULL OR (date_known = 1 AND date_saved > ?3))
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

    /// Gives a save its post, fetched for it on its own, and shows it in the saved list: the
    /// device now has what the list would have sent.
    pub fn link_post(
        executor: &impl QueryExecutor,
        saved: RowId,
        post: RowId,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE reader_saved_posts SET post_id = ?2, listed = 1
            WHERE rowid = ?1 AND (post_id IS NOT ?2 OR listed != 1)",
            params![saved, post],
        )?;
        Ok(())
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

    /// Deletes the account's rows that want the post unsaved. For a reader whose saves are only on
    /// this device: no server has them, so an unsaved row has nothing left to say. Returns how
    /// many it deleted.
    pub fn delete_unsaved(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<u64, SqliteDbError> {
        let deleted = executor.execute(
            "DELETE FROM reader_saved_posts WHERE account_id = ?1 AND saved = 0",
            [account],
        )?;
        Ok(deleted as u64)
    }

    /// Deletes every saved post row of the account, such as a signed-out reader's once an
    /// account has taken them over. The posts stay. Returns how many it deleted.
    pub fn delete_all(executor: &impl QueryExecutor, account: RowId) -> Result<u64, SqliteDbError> {
        let deleted = executor.execute(
            "DELETE FROM reader_saved_posts WHERE account_id = ?1",
            [account],
        )?;
        Ok(deleted as u64)
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

    /// The saved list: rows saved and listed, newest save first, with their posts.
    pub fn select_list(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<Vec<DbReaderSavedListItem>, SqliteDbError> {
        Self::select_list_ordered(
            executor,
            account,
            ReaderSavedOrderBy::Date,
            ReaderStreamOrder::Desc,
        )
    }

    /// The saved list in another order: by when each post was saved or published, newest or
    /// oldest first. A save without that date comes last either way.
    pub fn select_list_ordered(
        executor: &impl QueryExecutor,
        account: RowId,
        order_by: ReaderSavedOrderBy,
        order: ReaderStreamOrder,
    ) -> Result<Vec<DbReaderSavedListItem>, SqliteDbError> {
        let direction = match order {
            ReaderStreamOrder::Desc => "DESC",
            ReaderStreamOrder::Asc => "ASC",
        };
        let order = match order_by {
            ReaderSavedOrderBy::Date => format!(
                "s.date_saved IS NULL, s.date_saved {direction}, s.seq {direction}, s.rowid {direction}"
            ),
            ReaderSavedOrderBy::DatePublished => {
                format!("p.date IS NULL, p.date {direction}, s.rowid {direction}")
            }
        };
        let sql = format!(
            "SELECT {ROW_COLUMNS}, {OFFLINE_COLUMNS}, {} FROM reader_saved_posts s
            LEFT JOIN reader_posts p ON p.rowid = s.post_id
            LEFT JOIN reader_saved_offline o ON o.saved_post_id = s.rowid
            WHERE s.account_id = ?1 AND s.saved = 1 AND s.listed = 1
            ORDER BY {order}",
            post_columns(PostContent::Included)
        );
        let mut statement = executor.prepare(&sql)?;
        let rows = statement.query_map([account], |row| {
            Ok((|| -> Result<DbReaderSavedListItem, SqliteDbError> {
                let saved = saved_row_from_row(row)?;
                let post = match saved.post_row_id {
                    Some(_) => Some(post_from_row(row)?.1),
                    None => None,
                };
                Ok(DbReaderSavedListItem {
                    row: saved,
                    post,
                    offline: offline_from_row(row, ROW_COLUMN_COUNT)?,
                })
            })())
        })?;
        collect_rows(rows.map(|row| row.map_err(SqliteDbError::from).and_then(|row| row)))
    }

    /// The account's row for a key, whatever the user wants for it, with its post's row.
    pub fn select_row_id_for_key(
        executor: &impl QueryExecutor,
        account: RowId,
        key: &ReaderSavedKey,
    ) -> Result<Option<(RowId, Option<RowId>)>, SqliteDbError> {
        Ok(
            Self::select_rows_for_keys(executor, account, std::slice::from_ref(key))?
                .first()
                .map(|row| (row.row_id, row.post_row_id)),
        )
    }
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
