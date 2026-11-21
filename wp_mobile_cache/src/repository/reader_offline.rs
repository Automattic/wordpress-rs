//! What this device has of each saved post for reading with no connection: whether its content
//! was fetched, and which of its images are stored as files.

use crate::{
    RowId, SqliteDbError,
    db_types::reader::{
        DbReaderOfflineImages, DbReaderSavedOffline, reader_date_from_text, reader_date_text,
        reader_saved_key_from_text,
    },
    repository::QueryExecutor,
};
use rusqlite::{OptionalExtension, Row, params};
use std::collections::HashSet;
use wp_api::date::WpGmtDateTime;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_saved::{ReaderFeedId, ReaderSavedKey};

/// The columns of [`DbReaderSavedOffline`], for a query that joins `reader_saved_offline` as `o`.
pub(crate) const OFFLINE_COLUMNS: &str = "COALESCE(o.fetched, 0), COALESCE(o.downloading, 0), o.error, COALESCE(o.attempts, 0), \
    o.attempted_at, COALESCE(o.images, 0)";

/// Reads [`OFFLINE_COLUMNS`], which start at column `first`.
pub(crate) fn offline_from_row(
    row: &Row,
    first: usize,
) -> Result<DbReaderSavedOffline, SqliteDbError> {
    Ok(DbReaderSavedOffline {
        fetched: row.get(first)?,
        downloading: row.get(first + 1)?,
        error: row.get(first + 2)?,
        attempts: row.get(first + 3)?,
        attempted_at: row
            .get::<_, Option<String>>(first + 4)?
            .map(|text| reader_date_from_text(&text))
            .transpose()?,
        images: DbReaderOfflineImages::from_number(row.get(first + 5)?),
    })
}

/// A save whose content this device doesn't have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbReaderSaveWithoutContent {
    /// The save's row.
    pub row_id: RowId,
    pub key: ReaderSavedKey,
    /// The site of the post the cache has for it, when it has one.
    pub site_id: Option<WpComSiteId>,
    /// The feed of the post the cache has for it. A feed item's key doesn't say its feed.
    pub feed_id: Option<ReaderFeedId>,
    pub offline: DbReaderSavedOffline,
}

/// A saved post to store the images of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbReaderPostForImages {
    /// The post's row.
    pub post: RowId,
    pub content: Option<String>,
    pub featured_image: Option<String>,
    pub display_image_url: Option<String>,
    pub site_is_private: bool,
    pub images: DbReaderOfflineImages,
}

/// An image of a saved post, as the cache knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbReaderOfflineFile {
    /// The image's address, as the post has it.
    pub url: String,
    /// The file's name in the directory, or `None` when the server said the image can't be had.
    pub file_name: Option<String>,
    pub bytes: u64,
}

/// What [`ReaderOfflineRepository::record_files_making_room`] forgot to make room.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DbReaderRoomMade {
    /// The posts whose images were forgotten, in the order they were.
    pub posts: Vec<RowId>,
    /// The files no post has any more, for the caller to delete.
    pub unused_files: Vec<String>,
}

/// The saves of an account, newest first: the saved list's own order.
const NEWEST_SAVE_FIRST: &str = "s.date_saved IS NULL, s.date_saved DESC, s.seq DESC, s.rowid DESC";

/// A save has its content when its post has some, or was stored in full and has none to give.
const HAS_CONTENT: &str = "(p.content IS NOT NULL OR COALESCE(o.fetched, 0) = 1)";

/// Saved posts' content and images, for reading with no connection.
pub struct ReaderOfflineRepository;

impl ReaderOfflineRepository {
    /// What the device has of a save.
    pub fn select(
        executor: &impl QueryExecutor,
        saved: RowId,
    ) -> Result<DbReaderSavedOffline, SqliteDbError> {
        let sql = format!(
            "SELECT {OFFLINE_COLUMNS} FROM reader_saved_offline o WHERE o.saved_post_id = ?1"
        );
        let mut statement = executor.prepare(&sql)?;
        let found = statement
            .query_row([saved], |row| Ok(offline_from_row(row, 0)))
            .optional()?;
        Ok(found.transpose()?.unwrap_or_default())
    }

    fn ensure(executor: &impl QueryExecutor, saved: RowId) -> Result<(), SqliteDbError> {
        executor.execute(
            "INSERT OR IGNORE INTO reader_saved_offline (saved_post_id) VALUES (?1)",
            [saved],
        )?;
        Ok(())
    }

    /// Records that a save's post was stored in full, and so that its last failure is over.
    pub fn set_fetched(executor: &impl QueryExecutor, saved: RowId) -> Result<(), SqliteDbError> {
        Self::ensure(executor, saved)?;
        executor.execute(
            "UPDATE reader_saved_offline
            SET fetched = 1, downloading = 0, error = NULL, attempts = 0, attempted_at = NULL
            WHERE saved_post_id = ?1 AND (fetched != 1 OR downloading != 0 OR error IS NOT NULL
                OR attempts != 0 OR attempted_at IS NOT NULL)",
            [saved],
        )?;
        Ok(())
    }

    /// Records that every save of `post` has its post stored in full.
    pub fn set_fetched_for_post(
        executor: &impl QueryExecutor,
        post: RowId,
    ) -> Result<(), SqliteDbError> {
        for saved in Self::saves_of_post(executor, post)? {
            Self::set_fetched(executor, saved)?;
        }
        Ok(())
    }

    /// Marks a save as being fetched, or not.
    pub fn set_downloading(
        executor: &impl QueryExecutor,
        saved: RowId,
        downloading: bool,
    ) -> Result<(), SqliteDbError> {
        Self::ensure(executor, saved)?;
        executor.execute(
            "UPDATE reader_saved_offline SET downloading = ?2
            WHERE saved_post_id = ?1 AND downloading != ?2",
            params![saved, downloading],
        )?;
        Ok(())
    }

    /// Records a try to get a save's content that the server answered with a failure.
    pub fn record_failure(
        executor: &impl QueryExecutor,
        saved: RowId,
        error: &str,
        now: &WpGmtDateTime,
    ) -> Result<(), SqliteDbError> {
        Self::ensure(executor, saved)?;
        executor.execute(
            "UPDATE reader_saved_offline
            SET downloading = 0, error = ?2, attempts = attempts + 1, attempted_at = ?3
            WHERE saved_post_id = ?1",
            params![saved, error, reader_date_text(now)],
        )?;
        Ok(())
    }

    /// Forgets a save's failures, so that it's tried again straight away.
    pub fn forget_failures(
        executor: &impl QueryExecutor,
        saved: RowId,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE reader_saved_offline SET error = NULL, attempts = 0, attempted_at = NULL
            WHERE saved_post_id = ?1 AND (error IS NOT NULL OR attempts != 0)",
            [saved],
        )?;
        Ok(())
    }

    /// Marks nothing as being fetched: a fetch that was running when the app last stopped isn't.
    pub fn reset_downloading(executor: &impl QueryExecutor) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE reader_saved_offline SET downloading = 0 WHERE downloading != 0",
            [],
        )?;
        Ok(())
    }

    fn saves_of_post(
        executor: &impl QueryExecutor,
        post: RowId,
    ) -> Result<Vec<RowId>, SqliteDbError> {
        let mut statement =
            executor.prepare("SELECT rowid FROM reader_saved_posts WHERE post_id = ?1")?;
        let rows = statement.query_map([post], |row| row.get(0))?;
        rows.map(|row| row.map_err(SqliteDbError::from)).collect()
    }

    /// Records whether the images of a post are stored, on every save of it.
    pub fn set_images(
        executor: &impl QueryExecutor,
        post: RowId,
        images: DbReaderOfflineImages,
    ) -> Result<(), SqliteDbError> {
        for saved in Self::saves_of_post(executor, post)? {
            Self::ensure(executor, saved)?;
            executor.execute(
                "UPDATE reader_saved_offline SET images = ?2
                WHERE saved_post_id = ?1 AND images != ?2",
                params![saved, images.number()],
            )?;
        }
        Ok(())
    }

    /// Marks the saves of a post as having their images fetched, or not.
    pub fn set_downloading_for_post(
        executor: &impl QueryExecutor,
        post: RowId,
        downloading: bool,
    ) -> Result<(), SqliteDbError> {
        for saved in Self::saves_of_post(executor, post)? {
            Self::set_downloading(executor, saved, downloading)?;
        }
        Ok(())
    }

    /// How many of the account's saves have no content on this device.
    pub fn count_without_content(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<u64, SqliteDbError> {
        let sql = format!(
            "SELECT COUNT(*) FROM reader_saved_posts s
            LEFT JOIN reader_posts p ON p.rowid = s.post_id
            LEFT JOIN reader_saved_offline o ON o.saved_post_id = s.rowid
            WHERE s.account_id = ?1 AND s.saved = 1 AND NOT {HAS_CONTENT}"
        );
        let mut statement = executor.prepare(&sql)?;
        Ok(statement.query_row([account], |row| row.get(0))?)
    }

    /// The account's saves that have no content on this device, newest first.
    pub fn select_without_content(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<Vec<DbReaderSaveWithoutContent>, SqliteDbError> {
        let sql = format!(
            "SELECT s.rowid, s.key, p.site_id, p.feed_id, {OFFLINE_COLUMNS}
            FROM reader_saved_posts s
            LEFT JOIN reader_posts p ON p.rowid = s.post_id
            LEFT JOIN reader_saved_offline o ON o.saved_post_id = s.rowid
            WHERE s.account_id = ?1 AND s.saved = 1 AND NOT {HAS_CONTENT}
            ORDER BY {NEWEST_SAVE_FIRST}"
        );
        let mut statement = executor.prepare(&sql)?;
        let rows = statement.query_map([account], |row| {
            Ok((|| -> Result<DbReaderSaveWithoutContent, SqliteDbError> {
                Ok(DbReaderSaveWithoutContent {
                    row_id: row.get(0)?,
                    key: reader_saved_key_from_text(&row.get::<_, String>(1)?)?,
                    site_id: row.get::<_, Option<u64>>(2)?.map(WpComSiteId),
                    feed_id: row.get::<_, Option<u64>>(3)?.map(ReaderFeedId),
                    offline: offline_from_row(row, 4)?,
                })
            })())
        })?;
        rows.map(|row| row.map_err(SqliteDbError::from).and_then(|row| row))
            .collect()
    }

    /// How many of the account's saves have their content, and how many of those their images.
    pub fn count_ready(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<(u64, u64), SqliteDbError> {
        let sql = format!(
            "SELECT COUNT(*), COALESCE(SUM(COALESCE(o.images, 0) = 1), 0)
            FROM reader_saved_posts s
            LEFT JOIN reader_posts p ON p.rowid = s.post_id
            LEFT JOIN reader_saved_offline o ON o.saved_post_id = s.rowid
            WHERE s.account_id = ?1 AND s.saved = 1 AND {HAS_CONTENT}"
        );
        let mut statement = executor.prepare(&sql)?;
        Ok(statement.query_row([account], |row| Ok((row.get(0)?, row.get(1)?)))?)
    }

    /// The account's saved posts that have their content, newest save first, each once. They're
    /// the posts whose images can be stored.
    pub fn select_posts_for_images(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<Vec<DbReaderPostForImages>, SqliteDbError> {
        let sql = format!(
            "SELECT p.rowid, p.content, p.featured_image, p.display_image_url,
                COALESCE(p.site_is_private, 0), COALESCE(o.images, 0)
            FROM reader_saved_posts s
            JOIN reader_posts p ON p.rowid = s.post_id
            LEFT JOIN reader_saved_offline o ON o.saved_post_id = s.rowid
            WHERE s.account_id = ?1 AND s.saved = 1 AND {HAS_CONTENT}
            ORDER BY {NEWEST_SAVE_FIRST}"
        );
        let mut statement = executor.prepare(&sql)?;
        let rows = statement.query_map([account], |row| {
            Ok(DbReaderPostForImages {
                post: row.get(0)?,
                content: row.get(1)?,
                featured_image: row.get(2)?,
                display_image_url: row.get(3)?,
                site_is_private: row.get(4)?,
                images: DbReaderOfflineImages::from_number(row.get(5)?),
            })
        })?;
        let mut seen = HashSet::new();
        let mut posts = Vec::new();
        for post in rows {
            let post = post?;
            if seen.insert(post.post) {
                posts.push(post);
            }
        }
        Ok(posts)
    }

    /// The images of a post the cache knows about: the stored ones, and the ones the server said
    /// can't be had.
    pub fn select_files(
        executor: &impl QueryExecutor,
        post: RowId,
    ) -> Result<Vec<DbReaderOfflineFile>, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT url, file_name, bytes FROM reader_offline_files WHERE post_id = ?1 ORDER BY rowid",
        )?;
        let rows = statement.query_map([post], |row| {
            Ok(DbReaderOfflineFile {
                url: row.get(0)?,
                file_name: row.get(1)?,
                bytes: row.get(2)?,
            })
        })?;
        rows.map(|row| row.map_err(SqliteDbError::from)).collect()
    }

    /// The file an image is already stored as, for another post of the account, and its size.
    pub fn select_stored_file(
        executor: &impl QueryExecutor,
        account: RowId,
        url: &str,
    ) -> Result<Option<(String, u64)>, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT f.file_name, f.bytes FROM reader_offline_files f
            JOIN reader_posts p ON p.rowid = f.post_id
            WHERE p.account_id = ?1 AND f.url = ?2 AND f.file_name IS NOT NULL
            LIMIT 1",
        )?;
        Ok(statement
            .query_row(params![account, url], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()?)
    }

    /// Records an image of a post: the file it's stored as, or `None` for the file when the
    /// server said it can't be had.
    pub fn record_file(
        executor: &impl QueryExecutor,
        post: RowId,
        url: &str,
        file_name: Option<&str>,
        bytes: u64,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "INSERT INTO reader_offline_files (post_id, url, file_name, bytes)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(post_id, url) DO UPDATE SET
                file_name = excluded.file_name, bytes = excluded.bytes",
            params![post, url, file_name, bytes],
        )?;
        Ok(())
    }

    /// How many bytes the stored files take, each file counted once. It counts every reader's
    /// files: they share the directory.
    pub fn stored_bytes(executor: &impl QueryExecutor) -> Result<u64, SqliteDbError> {
        Self::stored_bytes_without(executor, &[])
    }

    /// How many bytes the stored files would take once `posts` had none: a file another post has
    /// still counts. Nothing is removed.
    pub fn stored_bytes_without(
        executor: &impl QueryExecutor,
        posts: &[RowId],
    ) -> Result<u64, SqliteDbError> {
        Self::stored_bytes_with(executor, &[], posts)
    }

    /// How many bytes the stored files would take once `files` were recorded too and `without`
    /// had none: a file another post has still counts, and each file counts once. Nothing is
    /// changed.
    pub fn stored_bytes_with(
        executor: &impl QueryExecutor,
        files: &[DbReaderOfflineFile],
        without: &[RowId],
    ) -> Result<u64, SqliteDbError> {
        let named: Vec<(&String, &u64)> = files
            .iter()
            .filter_map(|file| Some((file.file_name.as_ref()?, &file.bytes)))
            .collect();
        let placeholders = vec!["?"; without.len()].join(", ");
        let adding = " UNION ALL SELECT ?, ?".repeat(named.len());
        let mut statement = executor.prepare(&format!(
            "SELECT COALESCE(SUM(bytes), 0) FROM (
                SELECT MAX(bytes) AS bytes FROM (
                    SELECT file_name, bytes FROM reader_offline_files
                    WHERE file_name IS NOT NULL AND post_id NOT IN ({placeholders})
                    {adding}
                )
                GROUP BY file_name
            )"
        ))?;
        let mut parameters: Vec<&dyn rusqlite::ToSql> = Vec::new();
        parameters.extend(without.iter().map(|post| post as &dyn rusqlite::ToSql));
        for (name, bytes) in &named {
            parameters.push(*name);
            parameters.push(*bytes);
        }
        Ok(statement.query_row(parameters.as_slice(), |row| row.get(0))?)
    }

    /// Records images of `post`, then forgets the images of the `older` posts, in that order, as
    /// [`Self::delete_files_of_post`] does, until the stored files take at most `max_bytes`.
    /// Returns what was forgotten, or `None` when the files still take more. Call it in a
    /// transaction, and roll that back on `None`: the rows have changed all the same.
    pub fn record_files_making_room(
        executor: &impl QueryExecutor,
        post: RowId,
        files: &[DbReaderOfflineFile],
        older: &[RowId],
        max_bytes: u64,
    ) -> Result<Option<DbReaderRoomMade>, SqliteDbError> {
        for file in files {
            Self::record_file(
                executor,
                post,
                &file.url,
                file.file_name.as_deref(),
                file.bytes,
            )?;
        }
        let mut made = DbReaderRoomMade::default();
        for older in older {
            if Self::stored_bytes(executor)? <= max_bytes {
                break;
            }
            made.unused_files.extend(Self::delete_files_of_post(
                executor,
                *older,
                DbReaderOfflineImages::NoRoom,
            )?);
            made.posts.push(*older);
        }
        Ok((Self::stored_bytes(executor)? <= max_bytes).then_some(made))
    }

    /// The names of every stored file, for any reader.
    pub fn select_file_names(
        executor: &impl QueryExecutor,
    ) -> Result<HashSet<String>, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT DISTINCT file_name FROM reader_offline_files WHERE file_name IS NOT NULL",
        )?;
        let rows = statement.query_map([], |row| row.get(0))?;
        rows.map(|row| row.map_err(SqliteDbError::from)).collect()
    }

    /// Forgets the images of a post, and records why on every save of it: `images` is
    /// [`DbReaderOfflineImages::NoRoom`] when they went to make room, and
    /// [`DbReaderOfflineImages::NotStored`] otherwise. Returns the files no post has any more,
    /// for the caller to delete.
    pub fn delete_files_of_post(
        executor: &impl QueryExecutor,
        post: RowId,
        images: DbReaderOfflineImages,
    ) -> Result<Vec<String>, SqliteDbError> {
        let names = Self::file_names_where(executor, "post_id = ?1", post)?;
        executor.execute(
            "DELETE FROM reader_offline_files WHERE post_id = ?1",
            [post],
        )?;
        Self::set_images(executor, post, images)?;
        Self::unused(executor, names)
    }

    /// Forgets every stored image of the account. Returns the files no post has any more, for
    /// the caller to delete.
    pub fn delete_all_files(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<Vec<String>, SqliteDbError> {
        const POSTS: &str = "post_id IN (SELECT rowid FROM reader_posts WHERE account_id = ?1)";
        let names = Self::file_names_where(executor, POSTS, account)?;
        executor.execute(
            &format!("DELETE FROM reader_offline_files WHERE {POSTS}"),
            [account],
        )?;
        executor.execute(
            "UPDATE reader_saved_offline SET images = 0
            WHERE images != 0 AND saved_post_id IN (
                SELECT rowid FROM reader_saved_posts WHERE account_id = ?1
            )",
            [account],
        )?;
        Self::unused(executor, names)
    }

    /// Forgets the images of the posts the account no longer saves. Returns the files no post has
    /// any more, for the caller to delete.
    pub fn delete_files_of_unsaved_posts(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<Vec<String>, SqliteDbError> {
        const UNSAVED: &str = "post_id IN (
            SELECT p.rowid FROM reader_posts p
            WHERE p.account_id = ?1 AND NOT EXISTS (
                SELECT 1 FROM reader_saved_posts s WHERE s.post_id = p.rowid AND s.saved = 1
            )
        )";
        let names = Self::file_names_where(executor, UNSAVED, account)?;
        let deleted = executor.execute(
            &format!("DELETE FROM reader_offline_files WHERE {UNSAVED}"),
            [account],
        )?;
        if deleted == 0 {
            return Ok(Vec::new());
        }
        // A row that still wants the post unsaved, until the server agrees, no longer has them.
        executor.execute(
            &format!(
                "UPDATE reader_saved_offline SET images = 0
                WHERE images != 0 AND saved_post_id IN (
                    SELECT rowid FROM reader_saved_posts WHERE {UNSAVED}
                )"
            ),
            [account],
        )?;
        Self::unused(executor, names)
    }

    fn file_names_where(
        executor: &impl QueryExecutor,
        condition: &str,
        parameter: RowId,
    ) -> Result<Vec<String>, SqliteDbError> {
        let mut statement = executor.prepare(&format!(
            "SELECT DISTINCT file_name FROM reader_offline_files
            WHERE file_name IS NOT NULL AND {condition}"
        ))?;
        let rows = statement.query_map([parameter], |row| row.get(0))?;
        rows.map(|row| row.map_err(SqliteDbError::from)).collect()
    }

    /// The files of `names` that no row names any more.
    fn unused(
        executor: &impl QueryExecutor,
        names: Vec<String>,
    ) -> Result<Vec<String>, SqliteDbError> {
        let used = Self::select_file_names(executor)?;
        Ok(names
            .into_iter()
            .filter(|name| !used.contains(name))
            .collect())
    }
}

#[cfg(test)]
#[path = "reader_offline_tests.rs"]
mod tests;
