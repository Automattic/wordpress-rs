use rusqlite::types::{FromSqlError, Type};
use rusqlite::{ErrorCode, OptionalExtension, Row, ffi};
use wp_api::{
    posts::{PostCommentStatus, PostFormat, PostId, PostPingStatus},
    terms::TermId,
    users::UserId,
};
use wp_mobile_sqlite::{ColumnIndex, RowExt};

use crate::{
    LocalContentError,
    db_types::{
        working_copy::{
            DbPageWorkingCopy, DbPostWorkingCopy, FeaturedImage, PageFieldsColumn, PageWorkingCopy,
            PostFieldsColumn, PostWorkingCopy, PublishDate, StatusSetting, Tag, WorkingCopyColumn,
            WorkingCopyCommon, WorkingCopyId, WorkingCopyKind, date_from_timestamp,
        },
        working_copy_target::{
            RemotePostReference, SiteColumn, SiteIdentity, WorkingCopyTarget,
            WorkingCopyTargetColumn,
        },
    },
    repository::{InTransaction, QueryExecutor},
};

const WORKING_COPIES: &str = "working_copies";
const POST_FIELDS: &str = "post_working_copy_fields";
const PAGE_FIELDS: &str = "page_working_copy_fields";
const TARGETS: &str = "working_copy_targets";
const SITES: &str = "sites";

/// Repository for working copies and their targets.
///
/// A working copy spans its row in `working_copies`, the fields table for its
/// kind, and its row in `working_copy_targets`, which references a row in
/// `sites`. Writes that touch more than one of them require a transaction, so a
/// working copy is never stored without its fields or its target.
pub struct WorkingCopyRepository;

impl WorkingCopyRepository {
    /// Insert a post working copy and its target under a newly generated ID.
    ///
    /// Fails with `RemotePostAlreadyHasWorkingCopy` if the target's remote post
    /// already has a working copy.
    pub fn insert_post(
        &self,
        transaction: &impl InTransaction,
        working_copy: &PostWorkingCopy,
        target: &WorkingCopyTarget,
    ) -> Result<WorkingCopyId, LocalContentError> {
        let (id, rowid) = insert_common(transaction, WorkingCopyKind::Post, &working_copy.common)?;
        transaction.execute(
            &format!(
                "INSERT INTO {POST_FIELDS} (working_copy_rowid, category_ids, tags, format, sticky)
                 VALUES (?, ?, ?, ?, ?)"
            ),
            rusqlite::params![
                rowid,
                categories_to_json(&working_copy.categories)?,
                tags_to_json(&working_copy.tags)?,
                working_copy.format.as_str(),
                working_copy.sticky,
            ],
        )?;
        insert_target(transaction, rowid, target)?;
        Ok(id)
    }

    /// Insert a page working copy and its target under a newly generated ID.
    ///
    /// Fails with `RemotePostAlreadyHasWorkingCopy` if the target's remote post
    /// already has a working copy.
    pub fn insert_page(
        &self,
        transaction: &impl InTransaction,
        working_copy: &PageWorkingCopy,
        target: &WorkingCopyTarget,
    ) -> Result<WorkingCopyId, LocalContentError> {
        let (id, rowid) = insert_common(transaction, WorkingCopyKind::Page, &working_copy.common)?;
        transaction.execute(
            &format!("INSERT INTO {PAGE_FIELDS} (working_copy_rowid, parent_id) VALUES (?, ?)"),
            rusqlite::params![rowid, working_copy.parent.map(|parent| parent.0)],
        )?;
        insert_target(transaction, rowid, target)?;
        Ok(id)
    }

    /// Replace the fields of a post working copy and refresh its `updated_at`.
    /// The target is left unchanged.
    ///
    /// Returns `false` if no post working copy has the given ID.
    pub fn update_post(
        &self,
        transaction: &impl InTransaction,
        id: &WorkingCopyId,
        working_copy: &PostWorkingCopy,
    ) -> Result<bool, LocalContentError> {
        let Some(rowid) =
            update_common(transaction, WorkingCopyKind::Post, id, &working_copy.common)?
        else {
            return Ok(false);
        };
        transaction.execute(
            &format!(
                "UPDATE {POST_FIELDS} SET category_ids = ?, tags = ?, format = ?, sticky = ?
                 WHERE working_copy_rowid = ?"
            ),
            rusqlite::params![
                categories_to_json(&working_copy.categories)?,
                tags_to_json(&working_copy.tags)?,
                working_copy.format.as_str(),
                working_copy.sticky,
                rowid,
            ],
        )?;
        Ok(true)
    }

    /// Replace the fields of a page working copy and refresh its `updated_at`.
    /// The target is left unchanged.
    ///
    /// Returns `false` if no page working copy has the given ID.
    pub fn update_page(
        &self,
        transaction: &impl InTransaction,
        id: &WorkingCopyId,
        working_copy: &PageWorkingCopy,
    ) -> Result<bool, LocalContentError> {
        let Some(rowid) =
            update_common(transaction, WorkingCopyKind::Page, id, &working_copy.common)?
        else {
            return Ok(false);
        };
        transaction.execute(
            &format!("UPDATE {PAGE_FIELDS} SET parent_id = ? WHERE working_copy_rowid = ?"),
            rusqlite::params![working_copy.parent.map(|parent| parent.0), rowid],
        )?;
        Ok(true)
    }

    /// Record that a post working copy was pushed to `remote_post`: from now
    /// on it edits that post, based on the given `modified_gmt`. Its fields and
    /// `updated_at` are left unchanged.
    ///
    /// Returns `false` if no post working copy has the given ID. Fails with
    /// `RemotePostAlreadyHasWorkingCopy` if another working copy edits that
    /// post, and with `PushedToAnotherRemotePost` if this one edits a
    /// different post.
    pub fn record_post_push(
        &self,
        transaction: &impl InTransaction,
        id: &WorkingCopyId,
        remote_post: RemotePostReference,
    ) -> Result<bool, LocalContentError> {
        record_push(transaction, WorkingCopyKind::Post, id, remote_post)
    }

    /// Record that a page working copy was pushed to `remote_post`: from now
    /// on it edits that page, based on the given `modified_gmt`. Its fields and
    /// `updated_at` are left unchanged.
    ///
    /// Returns `false` if no page working copy has the given ID. Fails with
    /// `RemotePostAlreadyHasWorkingCopy` if another working copy edits that
    /// page, and with `PushedToAnotherRemotePost` if this one edits a
    /// different page.
    pub fn record_page_push(
        &self,
        transaction: &impl InTransaction,
        id: &WorkingCopyId,
        remote_post: RemotePostReference,
    ) -> Result<bool, LocalContentError> {
        record_push(transaction, WorkingCopyKind::Page, id, remote_post)
    }

    /// Delete a post working copy. Its fields and target are deleted with it.
    ///
    /// Returns `false` if no post working copy has the given ID.
    pub fn delete_post(
        &self,
        executor: &impl QueryExecutor,
        id: &WorkingCopyId,
    ) -> Result<bool, LocalContentError> {
        delete_working_copy(executor, WorkingCopyKind::Post, id)
    }

    /// Delete a page working copy. Its fields and target are deleted with it.
    ///
    /// Returns `false` if no page working copy has the given ID.
    pub fn delete_page(
        &self,
        executor: &impl QueryExecutor,
        id: &WorkingCopyId,
    ) -> Result<bool, LocalContentError> {
        delete_working_copy(executor, WorkingCopyKind::Page, id)
    }

    /// Delete every working copy targeting `site`, posts and pages, and the
    /// site's row. Their fields and targets are deleted with them.
    ///
    /// Returns the number of working copies deleted, 0 if no row matches `site`.
    pub fn delete_by_site(
        &self,
        transaction: &impl InTransaction,
        site: &SiteIdentity,
    ) -> Result<usize, LocalContentError> {
        let Some(site_rowid) = select_site_rowid(transaction, site)? else {
            return Ok(0);
        };
        let deleted = transaction.execute(
            &format!(
                "DELETE FROM {WORKING_COPIES}
                 WHERE rowid IN (SELECT working_copy_rowid FROM {TARGETS} WHERE site_rowid = ?)"
            ),
            [site_rowid],
        )?;
        transaction.execute(
            &format!("DELETE FROM {SITES} WHERE rowid = ?"),
            [site_rowid],
        )?;
        Ok(deleted)
    }

    /// Select a post working copy and its target by ID.
    ///
    /// Returns `Ok(None)` if no post working copy has the given ID, including
    /// when the ID belongs to a page.
    pub fn select_post(
        &self,
        executor: &impl QueryExecutor,
        id: &WorkingCopyId,
    ) -> Result<Option<DbPostWorkingCopy>, LocalContentError> {
        let Some(common_row) = select_common(executor, WorkingCopyKind::Post, id)? else {
            return Ok(None);
        };
        let mut stmt = executor.prepare(&format!(
            "SELECT * FROM {POST_FIELDS} WHERE working_copy_rowid = ?"
        ))?;
        let (categories, tags, format, sticky) = stmt.query_row([common_row.rowid], |row| {
            use PostFieldsColumn::*;
            let categories_json: String = row.get_column(CategoryIds)?;
            let tags_json: String = row.get_column(Tags)?;
            let format: String = row.get_column(Format)?;
            Ok((
                categories_from_json(&categories_json)
                    .map_err(|e| conversion_error(CategoryIds, e))?,
                tags_from_json(&tags_json).map_err(|e| conversion_error(Tags, e))?,
                format,
                row.get_column::<bool, _>(Sticky)?,
            ))
        })?;
        let target = select_target(executor, common_row.rowid)?;

        Ok(Some(DbPostWorkingCopy {
            id: common_row.id,
            working_copy: PostWorkingCopy {
                common: common_row.common,
                categories,
                tags,
                format: format
                    .parse::<PostFormat>()
                    .unwrap_or(PostFormat::Custom(format)),
                sticky,
            },
            target,
            created_at: common_row.created_at,
            updated_at: common_row.updated_at,
        }))
    }

    /// Select a page working copy and its target by ID.
    ///
    /// Returns `Ok(None)` if no page working copy has the given ID, including
    /// when the ID belongs to a post.
    pub fn select_page(
        &self,
        executor: &impl QueryExecutor,
        id: &WorkingCopyId,
    ) -> Result<Option<DbPageWorkingCopy>, LocalContentError> {
        let Some(common_row) = select_common(executor, WorkingCopyKind::Page, id)? else {
            return Ok(None);
        };
        let mut stmt = executor.prepare(&format!(
            "SELECT * FROM {PAGE_FIELDS} WHERE working_copy_rowid = ?"
        ))?;
        let parent: Option<i64> = stmt.query_row([common_row.rowid], |row| {
            row.get_column(PageFieldsColumn::ParentId)
        })?;
        let target = select_target(executor, common_row.rowid)?;

        Ok(Some(DbPageWorkingCopy {
            id: common_row.id,
            working_copy: PageWorkingCopy {
                common: common_row.common,
                parent: parent.map(PostId),
            },
            target,
            created_at: common_row.created_at,
            updated_at: common_row.updated_at,
        }))
    }

    /// Select the post working copy that edits remote post `post_id` on `site`.
    ///
    /// Returns `Ok(None)` if no post working copy edits it.
    pub fn select_post_by_remote_post(
        &self,
        executor: &impl QueryExecutor,
        site: &SiteIdentity,
        post_id: PostId,
    ) -> Result<Option<DbPostWorkingCopy>, LocalContentError> {
        match select_id_by_remote_post(executor, WorkingCopyKind::Post, site, post_id)? {
            Some(id) => self.select_post(executor, &id),
            None => Ok(None),
        }
    }

    /// Select the page working copy that edits remote page `post_id` on `site`.
    ///
    /// Returns `Ok(None)` if no page working copy edits it.
    pub fn select_page_by_remote_post(
        &self,
        executor: &impl QueryExecutor,
        site: &SiteIdentity,
        post_id: PostId,
    ) -> Result<Option<DbPageWorkingCopy>, LocalContentError> {
        match select_id_by_remote_post(executor, WorkingCopyKind::Page, site, post_id)? {
            Some(id) => self.select_page(executor, &id),
            None => Ok(None),
        }
    }

    /// Select every post working copy targeting `site`, most recently updated first.
    pub fn select_posts_by_site(
        &self,
        executor: &impl QueryExecutor,
        site: &SiteIdentity,
    ) -> Result<Vec<DbPostWorkingCopy>, LocalContentError> {
        select_ids_by_site(executor, WorkingCopyKind::Post, site)?
            .iter()
            .filter_map(|id| self.select_post(executor, id).transpose())
            .collect()
    }

    /// Select every page working copy targeting `site`, most recently updated first.
    pub fn select_pages_by_site(
        &self,
        executor: &impl QueryExecutor,
        site: &SiteIdentity,
    ) -> Result<Vec<DbPageWorkingCopy>, LocalContentError> {
        select_ids_by_site(executor, WorkingCopyKind::Page, site)?
            .iter()
            .filter_map(|id| self.select_page(executor, id).transpose())
            .collect()
    }

    /// Select every post working copy, whatever site it targets, most recently
    /// updated first.
    pub fn select_all_posts(
        &self,
        executor: &impl QueryExecutor,
    ) -> Result<Vec<DbPostWorkingCopy>, LocalContentError> {
        select_all_ids(executor, WorkingCopyKind::Post)?
            .iter()
            .filter_map(|id| self.select_post(executor, id).transpose())
            .collect()
    }

    /// Select every page working copy, whatever site it targets, most recently
    /// updated first.
    pub fn select_all_pages(
        &self,
        executor: &impl QueryExecutor,
    ) -> Result<Vec<DbPageWorkingCopy>, LocalContentError> {
        select_all_ids(executor, WorkingCopyKind::Page)?
            .iter()
            .filter_map(|id| self.select_page(executor, id).transpose())
            .collect()
    }
}

// Internal helpers

/// A `working_copies` row, read before the fields table for its kind.
struct CommonRow {
    rowid: i64,
    id: WorkingCopyId,
    common: WorkingCopyCommon,
    created_at: String,
    updated_at: String,
}

fn insert_common(
    transaction: &impl InTransaction,
    kind: WorkingCopyKind,
    common: &WorkingCopyCommon,
) -> Result<(WorkingCopyId, i64), LocalContentError> {
    let id = WorkingCopyId::generate();
    let (status, password) = common.status.to_columns();
    let mut stmt = transaction.prepare(&format!(
        "INSERT INTO {WORKING_COPIES} (
             uuid, kind, title, content, excerpt, slug, status, password, publish_date,
             author_id, featured_media_id, comment_status, ping_status
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING rowid"
    ))?;
    let rowid = stmt.query_row(
        rusqlite::params![
            id,
            kind.as_str(),
            common.title,
            common.content,
            common.excerpt,
            common.slug,
            status,
            password,
            common.publish_date.to_column(),
            common.author.map(|author| author.0),
            common.featured_image.to_column(),
            common.comment_status.as_ref().map(comment_status_str),
            common.ping_status.as_ref().map(ping_status_str),
        ],
        |row| row.get(0),
    )?;
    Ok((id, rowid))
}

/// Returns the updated row's `rowid`, or `None` if no working copy of `kind` has the ID.
fn update_common(
    transaction: &impl InTransaction,
    kind: WorkingCopyKind,
    id: &WorkingCopyId,
    common: &WorkingCopyCommon,
) -> Result<Option<i64>, LocalContentError> {
    let (status, password) = common.status.to_columns();
    let mut stmt = transaction.prepare(&format!(
        "UPDATE {WORKING_COPIES} SET
             title = ?, content = ?, excerpt = ?, slug = ?, status = ?, password = ?,
             publish_date = ?, author_id = ?, featured_media_id = ?, comment_status = ?,
             ping_status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE uuid = ? AND kind = ?
         RETURNING rowid"
    ))?;
    let rowid = stmt
        .query_row(
            rusqlite::params![
                common.title,
                common.content,
                common.excerpt,
                common.slug,
                status,
                password,
                common.publish_date.to_column(),
                common.author.map(|author| author.0),
                common.featured_image.to_column(),
                common.comment_status.as_ref().map(comment_status_str),
                common.ping_status.as_ref().map(ping_status_str),
                id,
                kind.as_str(),
            ],
            |row| row.get(0),
        )
        .optional()?;
    Ok(rowid)
}

/// The fields and target rows go with the working copy through `ON DELETE CASCADE`.
fn delete_working_copy(
    executor: &impl QueryExecutor,
    kind: WorkingCopyKind,
    id: &WorkingCopyId,
) -> Result<bool, LocalContentError> {
    let deleted = executor.execute(
        &format!("DELETE FROM {WORKING_COPIES} WHERE uuid = ? AND kind = ?"),
        rusqlite::params![id, kind.as_str()],
    )?;
    Ok(deleted > 0)
}

fn record_push(
    transaction: &impl InTransaction,
    kind: WorkingCopyKind,
    id: &WorkingCopyId,
    remote_post: RemotePostReference,
) -> Result<bool, LocalContentError> {
    let mut stmt = transaction.prepare(&format!(
        "SELECT w.rowid, t.remote_post_id FROM {WORKING_COPIES} w
         JOIN {TARGETS} t ON t.working_copy_rowid = w.rowid
         WHERE w.uuid = ? AND w.kind = ?"
    ))?;
    let Some((rowid, edited_post_id)) = stmt
        .query_row(rusqlite::params![id, kind.as_str()], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?))
        })
        .optional()?
    else {
        return Ok(false);
    };
    if let Some(edited_post_id) = edited_post_id.map(PostId)
        && edited_post_id != remote_post.post_id
    {
        return Err(LocalContentError::PushedToAnotherRemotePost {
            edited_post_id,
            pushed_post_id: remote_post.post_id,
        });
    }

    let mut update = transaction.prepare(&format!(
        "UPDATE {TARGETS} SET remote_post_id = ?, base_modified_gmt = ?
         WHERE working_copy_rowid = ?"
    ))?;
    let result = update.execute(rusqlite::params![
        remote_post.post_id.0,
        remote_post.base_modified_gmt.0.timestamp(),
        rowid,
    ]);
    remote_post_conflict(result, remote_post.post_id)?;
    Ok(true)
}

fn select_id_by_remote_post(
    executor: &impl QueryExecutor,
    kind: WorkingCopyKind,
    site: &SiteIdentity,
    post_id: PostId,
) -> Result<Option<WorkingCopyId>, LocalContentError> {
    let Some(site_rowid) = select_site_rowid(executor, site)? else {
        return Ok(None);
    };
    let mut stmt = executor.prepare(&format!(
        "SELECT w.uuid FROM {WORKING_COPIES} w
         JOIN {TARGETS} t ON t.working_copy_rowid = w.rowid
         WHERE w.kind = ? AND t.site_rowid = ? AND t.remote_post_id = ?"
    ))?;
    let id = stmt
        .query_row(
            rusqlite::params![kind.as_str(), site_rowid, post_id.0],
            |row| row.get(0),
        )
        .optional()?;
    Ok(id)
}

fn select_ids_by_site(
    executor: &impl QueryExecutor,
    kind: WorkingCopyKind,
    site: &SiteIdentity,
) -> Result<Vec<WorkingCopyId>, LocalContentError> {
    let Some(site_rowid) = select_site_rowid(executor, site)? else {
        return Ok(vec![]);
    };
    let mut stmt = executor.prepare(&format!(
        "SELECT w.uuid FROM {WORKING_COPIES} w
         JOIN {TARGETS} t ON t.working_copy_rowid = w.rowid
         WHERE w.kind = ? AND t.site_rowid = ?
         ORDER BY w.updated_at DESC, w.rowid DESC"
    ))?;
    let ids = stmt
        .query_map(rusqlite::params![kind.as_str(), site_rowid], |row| {
            row.get(0)
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

fn select_all_ids(
    executor: &impl QueryExecutor,
    kind: WorkingCopyKind,
) -> Result<Vec<WorkingCopyId>, LocalContentError> {
    let mut stmt = executor.prepare(&format!(
        "SELECT uuid FROM {WORKING_COPIES} WHERE kind = ?
         ORDER BY updated_at DESC, rowid DESC"
    ))?;
    let ids = stmt
        .query_map([kind.as_str()], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

fn select_common(
    executor: &impl QueryExecutor,
    kind: WorkingCopyKind,
    id: &WorkingCopyId,
) -> Result<Option<CommonRow>, LocalContentError> {
    let mut stmt = executor.prepare(&format!(
        "SELECT * FROM {WORKING_COPIES} WHERE uuid = ? AND kind = ?"
    ))?;
    let row = stmt
        .query_row(rusqlite::params![id, kind.as_str()], common_from_row)
        .optional()?;
    Ok(row)
}

fn common_from_row(row: &Row) -> rusqlite::Result<CommonRow> {
    use WorkingCopyColumn as C;

    let status: String = row.get_column(C::Status)?;
    let password: Option<String> = row.get_column(C::Password)?;
    let publish_date: Option<i64> = row.get_column(C::PublishDate)?;
    let author: Option<i64> = row.get_column(C::AuthorId)?;
    let featured_media_id: Option<i64> = row.get_column(C::FeaturedMediaId)?;
    let comment_status: Option<String> = row.get_column(C::CommentStatus)?;
    let ping_status: Option<String> = row.get_column(C::PingStatus)?;

    Ok(CommonRow {
        rowid: row.get_column(C::Rowid)?,
        id: row.get_column(C::Uuid)?,
        common: WorkingCopyCommon {
            title: row.get_column(C::Title)?,
            content: row.get_column(C::Content)?,
            excerpt: row.get_column(C::Excerpt)?,
            slug: row.get_column(C::Slug)?,
            status: StatusSetting::from_columns(&status, password)
                .map_err(|e| conversion_error(C::Status, e))?,
            publish_date: PublishDate::from_column(publish_date)
                .map_err(|e| conversion_error(C::PublishDate, e))?,
            author: author.map(UserId),
            featured_image: FeaturedImage::from_column(featured_media_id),
            comment_status: comment_status.as_deref().map(comment_status_from_str),
            ping_status: ping_status.as_deref().map(ping_status_from_str),
        },
        created_at: row.get_column(C::CreatedAt)?,
        updated_at: row.get_column(C::UpdatedAt)?,
    })
}

fn insert_target(
    transaction: &impl InTransaction,
    rowid: i64,
    target: &WorkingCopyTarget,
) -> Result<(), LocalContentError> {
    let site_rowid = select_or_insert_site(transaction, &target.site)?;
    let mut stmt = transaction.prepare(&format!(
        "INSERT INTO {TARGETS} (working_copy_rowid, site_rowid, remote_post_id, base_modified_gmt)
         VALUES (?, ?, ?, ?)"
    ))?;
    let result = stmt.execute(rusqlite::params![
        rowid,
        site_rowid,
        target.remote_post.map(|remote| remote.post_id.0),
        target
            .remote_post
            .map(|remote| remote.base_modified_gmt.0.timestamp()),
    ]);
    match target.remote_post {
        Some(remote_post) => remote_post_conflict(result, remote_post.post_id),
        None => {
            result?;
            Ok(())
        }
    }
}

/// Maps the result of writing a target that edits remote post `post_id`.
///
/// The only unique constraint a target write can break is the one on
/// `(site_rowid, remote_post_id)`, meaning another working copy edits the post.
fn remote_post_conflict(
    result: rusqlite::Result<usize>,
    post_id: PostId,
) -> Result<(), LocalContentError> {
    match result {
        Err(rusqlite::Error::SqliteFailure(err, _))
            if err.code == ErrorCode::ConstraintViolation
                && err.extended_code == ffi::SQLITE_CONSTRAINT_UNIQUE =>
        {
            Err(LocalContentError::RemotePostAlreadyHasWorkingCopy { post_id })
        }
        result => {
            result?;
            Ok(())
        }
    }
}

/// Returns the `sites` row `site` identifies, if there is one: the row with
/// its WordPress.com site ID, or else the row with its URL and no site ID.
fn select_site_rowid(
    executor: &impl QueryExecutor,
    site: &SiteIdentity,
) -> Result<Option<i64>, LocalContentError> {
    let (wordpress_com_site_id, url) = site.to_columns();
    let mut stmt = executor.prepare(&format!(
        "SELECT rowid FROM {SITES}
         WHERE wordpress_com_site_id = ?1 OR (wordpress_com_site_id IS NULL AND url = ?2)
         ORDER BY wordpress_com_site_id IS NULL
         LIMIT 1"
    ))?;
    let rowid = stmt
        .query_row(rusqlite::params![wordpress_com_site_id, url], |row| {
            row.get(0)
        })
        .optional()?;
    Ok(rowid)
}

/// Returns the `sites` row `site` identifies, inserting one if there isn't one
/// yet. An existing row takes `site`'s URL, and its site ID if it had none.
fn select_or_insert_site(
    transaction: &impl InTransaction,
    site: &SiteIdentity,
) -> Result<i64, LocalContentError> {
    let (wordpress_com_site_id, url) = site.to_columns();
    if let Some(rowid) = select_site_rowid(transaction, site)? {
        transaction.execute(
            &format!(
                "UPDATE {SITES}
                 SET wordpress_com_site_id = COALESCE(?, wordpress_com_site_id), url = ?
                 WHERE rowid = ?"
            ),
            rusqlite::params![wordpress_com_site_id, url, rowid],
        )?;
        return Ok(rowid);
    }

    let mut insert = transaction.prepare(&format!(
        "INSERT INTO {SITES} (wordpress_com_site_id, url) VALUES (?, ?) RETURNING rowid"
    ))?;
    Ok(
        insert.query_row(rusqlite::params![wordpress_com_site_id, url], |row| {
            row.get(0)
        })?,
    )
}

fn select_target(
    executor: &impl QueryExecutor,
    rowid: i64,
) -> Result<WorkingCopyTarget, LocalContentError> {
    let mut stmt = executor.prepare(&format!(
        "SELECT * FROM {TARGETS} WHERE working_copy_rowid = ?"
    ))?;
    let (site_rowid, remote_post) = stmt.query_row([rowid], |row| {
        use WorkingCopyTargetColumn::*;

        let site_rowid: i64 = row.get_column(SiteRowid)?;
        let remote_post_id: Option<i64> = row.get_column(RemotePostId)?;
        let base_modified_gmt: Option<i64> = row.get_column(BaseModifiedGmt)?;
        let remote_post = match (remote_post_id, base_modified_gmt) {
            (Some(post_id), Some(seconds)) => Some(RemotePostReference {
                post_id: PostId(post_id),
                base_modified_gmt: date_from_timestamp(seconds)
                    .map_err(|e| conversion_error(BaseModifiedGmt, e))?,
            }),
            (None, None) => None,
            _ => {
                return Err(conversion_error(
                    RemotePostId,
                    FromSqlError::Other(
                        "remote_post_id and base_modified_gmt must both be set or both be NULL"
                            .into(),
                    ),
                ));
            }
        };
        Ok((site_rowid, remote_post))
    })?;
    let site = select_site(executor, site_rowid)?;

    Ok(WorkingCopyTarget { site, remote_post })
}

fn select_site(
    executor: &impl QueryExecutor,
    site_rowid: i64,
) -> Result<SiteIdentity, LocalContentError> {
    let mut stmt = executor.prepare(&format!("SELECT * FROM {SITES} WHERE rowid = ?"))?;
    let site = stmt.query_row([site_rowid], |row| {
        use SiteColumn::*;

        let url: String = row.get_column(Url)?;
        SiteIdentity::from_columns(row.get_column(WordPressComSiteId)?, &url)
            .map_err(|e| conversion_error(Url, e))
    })?;
    Ok(site)
}

fn categories_to_json(categories: &[TermId]) -> Result<String, LocalContentError> {
    let ids: Vec<i64> = categories.iter().map(|category| category.0).collect();
    serde_json::to_string(&ids).map_err(json_error)
}

fn categories_from_json(json: &str) -> Result<Vec<TermId>, FromSqlError> {
    let ids: Vec<i64> = serde_json::from_str(json).map_err(|e| FromSqlError::Other(e.into()))?;
    Ok(ids.into_iter().map(TermId).collect())
}

fn tags_to_json(tags: &[Tag]) -> Result<String, LocalContentError> {
    serde_json::to_string(tags).map_err(json_error)
}

fn tags_from_json(json: &str) -> Result<Vec<Tag>, FromSqlError> {
    serde_json::from_str(json).map_err(|e| FromSqlError::Other(e.into()))
}

fn comment_status_str(status: &PostCommentStatus) -> &str {
    match status {
        PostCommentStatus::Open => "open",
        PostCommentStatus::Closed => "closed",
        PostCommentStatus::Custom(status) => status,
    }
}

fn comment_status_from_str(status: &str) -> PostCommentStatus {
    match status {
        "open" => PostCommentStatus::Open,
        "closed" => PostCommentStatus::Closed,
        status => PostCommentStatus::Custom(status.to_string()),
    }
}

fn ping_status_str(status: &PostPingStatus) -> &str {
    match status {
        PostPingStatus::Open => "open",
        PostPingStatus::Closed => "closed",
        PostPingStatus::Custom(status) => status,
    }
}

fn ping_status_from_str(status: &str) -> PostPingStatus {
    match status {
        "open" => PostPingStatus::Open,
        "closed" => PostPingStatus::Closed,
        status => PostPingStatus::Custom(status.to_string()),
    }
}

fn json_error(err: serde_json::Error) -> LocalContentError {
    LocalContentError::Sqlite {
        err_message: format!("Failed to serialize to JSON: {err}"),
    }
}

fn conversion_error(column: impl ColumnIndex, err: FromSqlError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(column.as_index(), Type::Text, Box::new(err))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        MIGRATIONS,
        db_types::working_copy::{StatusSetting, WorkingCopyId},
    };
    use rusqlite::Connection;
    use std::sync::Arc;
    use wp_api::{
        date::WpGmtDateTime,
        media::MediaId,
        parsed_url::ParsedUrl,
        posts::{PostCommentStatus, PostPingStatus},
        wp_com::WpComSiteId,
    };
    use wp_mobile_sqlite::MigrationManager;

    fn migrated_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory database");
        MigrationManager::new(&conn, &MIGRATIONS)
            .expect("Failed to create MigrationManager")
            .perform_migrations()
            .expect("Migrations should succeed");
        conn
    }

    fn date(seconds: i64) -> WpGmtDateTime {
        date_from_timestamp(seconds).expect("Timestamp should be in range")
    }

    fn common(title: &str) -> WorkingCopyCommon {
        WorkingCopyCommon {
            title: title.to_string(),
            content: format!("<!-- wp:paragraph --><p>{title}</p><!-- /wp:paragraph -->"),
            excerpt: String::new(),
            slug: None,
            status: StatusSetting::Draft { password: None },
            publish_date: PublishDate::Immediately,
            author: None,
            featured_image: FeaturedImage::None,
            comment_status: None,
            ping_status: None,
        }
    }

    /// A post with every optional or enum field set away from its default.
    fn detailed_post() -> PostWorkingCopy {
        PostWorkingCopy {
            common: WorkingCopyCommon {
                excerpt: "An excerpt".to_string(),
                slug: Some("a-slug".to_string()),
                status: StatusSetting::Custom {
                    status: "in-review".to_string(),
                    password: Some("secret".to_string()),
                },
                publish_date: PublishDate::At {
                    date: date(1_900_000_000),
                },
                author: Some(UserId(7)),
                featured_image: FeaturedImage::Remote {
                    media_id: MediaId(42),
                },
                comment_status: Some(PostCommentStatus::Closed),
                ping_status: Some(PostPingStatus::Custom("moderated".to_string())),
                ..common("Detailed post")
            },
            categories: vec![TermId(1), TermId(5)],
            tags: vec![
                Tag::Existing { id: TermId(3) },
                Tag::New {
                    name: "not on the site yet".to_string(),
                },
            ],
            format: PostFormat::Custom("theme-specific".to_string()),
            sticky: true,
        }
    }

    fn simple_post(title: &str) -> PostWorkingCopy {
        PostWorkingCopy {
            common: common(title),
            categories: vec![],
            tags: vec![],
            format: PostFormat::Standard,
            sticky: false,
        }
    }

    fn site(url: &str, wordpress_com_site_id: Option<u64>) -> SiteIdentity {
        SiteIdentity {
            url: Arc::new(ParsedUrl::parse(url).expect("URL should parse")),
            wordpress_com_site_id: wordpress_com_site_id.map(WpComSiteId),
        }
    }

    fn new_post_on(site: &SiteIdentity) -> WorkingCopyTarget {
        WorkingCopyTarget {
            site: site.clone(),
            remote_post: None,
        }
    }

    fn wordpress_com_target_editing(post_id: i64) -> WorkingCopyTarget {
        WorkingCopyTarget {
            site: site("https://example.wordpress.com", Some(123_456_789)),
            remote_post: Some(RemotePostReference {
                post_id: PostId(post_id),
                base_modified_gmt: date(1_700_000_000),
            }),
        }
    }

    fn self_hosted_new_post_target() -> WorkingCopyTarget {
        new_post_on(&site("https://example.com", None))
    }

    fn insert_post(
        conn: &mut Connection,
        working_copy: &PostWorkingCopy,
        target: &WorkingCopyTarget,
    ) -> WorkingCopyId {
        let tx = conn.transaction().expect("Failed to begin transaction");
        let id = WorkingCopyRepository
            .insert_post(&tx, working_copy, target)
            .expect("Insert should succeed");
        tx.commit().expect("Commit should succeed");
        id
    }

    fn insert_page(
        conn: &mut Connection,
        working_copy: &PageWorkingCopy,
        target: &WorkingCopyTarget,
    ) -> WorkingCopyId {
        let tx = conn.transaction().expect("Failed to begin transaction");
        let id = WorkingCopyRepository
            .insert_page(&tx, working_copy, target)
            .expect("Insert should succeed");
        tx.commit().expect("Commit should succeed");
        id
    }

    fn count_rows(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("Count should succeed")
    }

    fn column_names(conn: &Connection, table: &str) -> Vec<String> {
        conn.prepare(&format!("PRAGMA table_info({table})"))
            .expect("Failed to prepare PRAGMA query")
            .query_map([], |row| row.get::<_, String>(1))
            .expect("Failed to execute PRAGMA query")
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to collect column names")
    }

    /// The repository reads columns by index, so every column enum has to match
    /// the schema's column order.
    #[test]
    fn test_column_enums_match_schema() {
        let conn = migrated_connection();

        let columns = column_names(&conn, WORKING_COPIES);
        for (column, name) in [
            (WorkingCopyColumn::Rowid, "rowid"),
            (WorkingCopyColumn::Uuid, "uuid"),
            (WorkingCopyColumn::Title, "title"),
            (WorkingCopyColumn::Content, "content"),
            (WorkingCopyColumn::Excerpt, "excerpt"),
            (WorkingCopyColumn::Slug, "slug"),
            (WorkingCopyColumn::Status, "status"),
            (WorkingCopyColumn::Password, "password"),
            (WorkingCopyColumn::PublishDate, "publish_date"),
            (WorkingCopyColumn::AuthorId, "author_id"),
            (WorkingCopyColumn::FeaturedMediaId, "featured_media_id"),
            (WorkingCopyColumn::CommentStatus, "comment_status"),
            (WorkingCopyColumn::PingStatus, "ping_status"),
            (WorkingCopyColumn::CreatedAt, "created_at"),
            (WorkingCopyColumn::UpdatedAt, "updated_at"),
        ] {
            assert_eq!(columns[column.as_index()], name);
        }
        assert_eq!(columns[2], "kind");
        assert_eq!(columns.len(), 16);

        let columns = column_names(&conn, POST_FIELDS);
        for (column, name) in [
            (PostFieldsColumn::CategoryIds, "category_ids"),
            (PostFieldsColumn::Tags, "tags"),
            (PostFieldsColumn::Format, "format"),
            (PostFieldsColumn::Sticky, "sticky"),
        ] {
            assert_eq!(columns[column.as_index()], name);
        }
        assert_eq!(columns.len(), 5);

        let columns = column_names(&conn, PAGE_FIELDS);
        assert_eq!(columns[PageFieldsColumn::ParentId.as_index()], "parent_id");
        assert_eq!(columns.len(), 2);

        let columns = column_names(&conn, TARGETS);
        for (column, name) in [
            (WorkingCopyTargetColumn::SiteRowid, "site_rowid"),
            (WorkingCopyTargetColumn::RemotePostId, "remote_post_id"),
            (
                WorkingCopyTargetColumn::BaseModifiedGmt,
                "base_modified_gmt",
            ),
        ] {
            assert_eq!(columns[column.as_index()], name);
        }
        assert_eq!(columns.len(), 4);

        let columns = column_names(&conn, SITES);
        for (column, name) in [
            (SiteColumn::WordPressComSiteId, "wordpress_com_site_id"),
            (SiteColumn::Url, "url"),
        ] {
            assert_eq!(columns[column.as_index()], name);
        }
        assert_eq!(columns.len(), 3);
    }

    #[test]
    fn test_post_round_trips_every_field() {
        let mut conn = migrated_connection();
        let post = detailed_post();
        let target = wordpress_com_target_editing(99);

        let id = insert_post(&mut conn, &post, &target);
        let stored = WorkingCopyRepository
            .select_post(&conn, &id)
            .expect("Select should succeed")
            .expect("Inserted post should exist");

        assert_eq!(stored.id, id);
        assert_eq!(stored.working_copy, post);
        assert_eq!(stored.target, target);
    }

    #[test]
    fn test_page_round_trips_every_field() {
        let mut conn = migrated_connection();
        let page = PageWorkingCopy {
            common: common("A page"),
            parent: Some(PostId(3)),
        };
        let target = self_hosted_new_post_target();

        let id = insert_page(&mut conn, &page, &target);
        let stored = WorkingCopyRepository
            .select_page(&conn, &id)
            .expect("Select should succeed")
            .expect("Inserted page should exist");

        assert_eq!(stored.working_copy, page);
        assert_eq!(stored.target, target);
    }

    #[test]
    fn test_post_and_page_ids_are_not_interchangeable() {
        let mut conn = migrated_connection();
        let page_id = insert_page(
            &mut conn,
            &PageWorkingCopy {
                common: common("A page"),
                parent: None,
            },
            &self_hosted_new_post_target(),
        );

        let as_post = WorkingCopyRepository
            .select_post(&conn, &page_id)
            .expect("Select should succeed");
        let tx = conn.transaction().expect("Failed to begin transaction");
        let updated_as_post = WorkingCopyRepository
            .update_post(&tx, &page_id, &simple_post("Not a page"))
            .expect("Update should succeed");

        assert_eq!(as_post, None);
        assert!(!updated_as_post, "A page must not be updated as a post");
    }

    #[test]
    fn test_update_replaces_fields_and_keeps_the_target() {
        let mut conn = migrated_connection();
        let target = wordpress_com_target_editing(99);
        let id = insert_post(&mut conn, &detailed_post(), &target);
        let before = WorkingCopyRepository
            .select_post(&conn, &id)
            .expect("Select should succeed")
            .expect("Inserted post should exist");

        // Removing the password must clear the stored one.
        let edited = simple_post("Edited");
        let tx = conn.transaction().expect("Failed to begin transaction");
        let updated = WorkingCopyRepository
            .update_post(&tx, &id, &edited)
            .expect("Update should succeed");
        tx.commit().expect("Commit should succeed");
        let after = WorkingCopyRepository
            .select_post(&conn, &id)
            .expect("Select should succeed")
            .expect("Updated post should exist");
        let stored_password: Option<String> = conn
            .query_row(
                &format!("SELECT password FROM {WORKING_COPIES} WHERE uuid = ?"),
                [&id],
                |row| row.get(0),
            )
            .expect("Query should succeed");

        assert!(updated);
        assert_eq!(after.working_copy, edited);
        assert_eq!(after.target, target);
        assert_eq!(after.created_at, before.created_at);
        assert_eq!(stored_password, None);
    }

    #[test]
    fn test_update_returns_false_for_unknown_id() {
        let mut conn = migrated_connection();
        let tx = conn.transaction().expect("Failed to begin transaction");
        let updated = WorkingCopyRepository
            .update_post(
                &tx,
                &WorkingCopyId("00000000-0000-4000-8000-000000000000".to_string()),
                &simple_post("Title"),
            )
            .expect("Update should succeed");
        assert!(!updated);
    }

    /// The repository only writes consistent rows; the schema rejects the
    /// inconsistent ones in case anything else ever writes to these tables.
    #[test]
    fn test_schema_rejects_impossible_states() {
        let mut conn = migrated_connection();
        let id = insert_post(
            &mut conn,
            &simple_post("Title"),
            &self_hosted_new_post_target(),
        );

        let empty_status = conn.execute(
            &format!("UPDATE {WORKING_COPIES} SET status = '' WHERE uuid = ?"),
            [&id],
        );
        let private_with_password = conn.execute(
            &format!(
                "UPDATE {WORKING_COPIES} SET status = 'private', password = 'secret' WHERE uuid = ?"
            ),
            [&id],
        );
        let tags_not_an_array = conn.execute(
            &format!("UPDATE {POST_FIELDS} SET tags = '{{\"new\": {{\"name\": \"a\"}}}}'"),
            [],
        );
        let remote_post_without_base =
            conn.execute(&format!("UPDATE {TARGETS} SET remote_post_id = 5"), []);
        let two_sites_without_a_site_id_at_one_url = conn.execute(
            &format!("INSERT INTO {SITES} (url) SELECT url FROM {SITES}"),
            [],
        );

        for (case, result) in [
            ("an empty status", empty_status),
            ("a private post with a password", private_with_password),
            ("tags that aren't an array", tags_not_an_array),
            ("remote post without base", remote_post_without_base),
            (
                "two sites without a site ID at one URL",
                two_sites_without_a_site_id_at_one_url,
            ),
        ] {
            assert!(result.is_err(), "Schema should reject {case}");
        }
    }

    /// Relationships depend on their working copy, never the other way around,
    /// and deleting a site never takes a working copy with it.
    #[test]
    fn test_relationships_are_deleted_with_their_working_copy_and_not_without_one() {
        let mut conn = migrated_connection();
        let id = insert_post(
            &mut conn,
            &detailed_post(),
            &wordpress_com_target_editing(99),
        );

        let orphan_target = conn.execute(
            &format!(
                "INSERT INTO {TARGETS} (working_copy_rowid, site_rowid)
                 SELECT 12345, rowid FROM {SITES}"
            ),
            [],
        );
        let delete_targeted_site = conn.execute(&format!("DELETE FROM {SITES}"), []);
        conn.execute(
            &format!("DELETE FROM {WORKING_COPIES} WHERE uuid = ?"),
            [&id],
        )
        .expect("Delete should succeed");

        assert!(
            orphan_target.is_err(),
            "A target must not exist without a working copy"
        );
        assert!(
            delete_targeted_site.is_err(),
            "A site must not be deleted while a working copy targets it"
        );
        assert_eq!(count_rows(&conn, POST_FIELDS), 0);
        assert_eq!(count_rows(&conn, TARGETS), 0);
    }

    #[test]
    fn test_working_copies_are_listed_most_recently_updated_first() {
        let mut conn = migrated_connection();
        let site = self_hosted_new_post_target();
        // Created first and updated last, so creation order and update order disagree.
        let edited_last = insert_post(&mut conn, &simple_post("Edited last"), &site);
        let oldest = insert_post(&mut conn, &simple_post("Oldest"), &site);
        let newer = insert_post(&mut conn, &simple_post("Newer"), &site);
        let another_sites = insert_post(
            &mut conn,
            &simple_post("Another site's"),
            &wordpress_com_target_editing(99),
        );
        insert_page(
            &mut conn,
            &PageWorkingCopy {
                common: common("A page"),
                parent: None,
            },
            &site,
        );
        for (id, updated_at) in [
            (&oldest, "2026-01-01T00:00:00.000Z"),
            (&newer, "2026-02-01T00:00:00.000Z"),
            (&another_sites, "2026-02-15T00:00:00.000Z"),
            (&edited_last, "2026-03-01T00:00:00.000Z"),
        ] {
            conn.execute(
                &format!("UPDATE {WORKING_COPIES} SET updated_at = ? WHERE uuid = ?"),
                rusqlite::params![updated_at, id],
            )
            .expect("Update should succeed");
        }

        let ids = |posts: Vec<DbPostWorkingCopy>| -> Vec<WorkingCopyId> {
            posts.into_iter().map(|post| post.id).collect()
        };
        let by_site = WorkingCopyRepository
            .select_posts_by_site(&conn, &site.site)
            .expect("Select should succeed");
        let all = WorkingCopyRepository
            .select_all_posts(&conn)
            .expect("Select should succeed");

        assert_eq!(
            ids(by_site),
            vec![edited_last.clone(), newer.clone(), oldest.clone()]
        );
        assert_eq!(ids(all), vec![edited_last, another_sites, newer, oldest]);
    }

    #[test]
    fn test_a_remote_post_has_at_most_one_working_copy() {
        let mut conn = migrated_connection();
        let self_hosted = |url: &str, remote_post: Option<RemotePostReference>| WorkingCopyTarget {
            site: site(url, None),
            remote_post,
        };
        let editing_post_5 = Some(RemotePostReference {
            post_id: PostId(5),
            base_modified_gmt: date(1_700_000_000),
        });
        insert_post(
            &mut conn,
            &simple_post("First"),
            &self_hosted("https://example.com", editing_post_5),
        );

        let insert = |conn: &mut Connection, target: &WorkingCopyTarget| {
            let tx = conn.transaction().expect("Failed to begin transaction");
            let result = WorkingCopyRepository.insert_post(&tx, &simple_post("Another"), target);
            // A failed insert has already written the working copy's row, so
            // it's rolled back by dropping the transaction rather than committed.
            if result.is_ok() {
                tx.commit().expect("Commit should succeed");
            }
            result
        };
        let same_post = insert(
            &mut conn,
            &self_hosted("https://example.com", editing_post_5),
        );
        let same_post_differently_written_url = insert(
            &mut conn,
            &self_hosted("HTTPS://EXAMPLE.COM/", editing_post_5),
        );
        let new_posts = [
            insert(&mut conn, &self_hosted("https://example.com", None)),
            insert(&mut conn, &self_hosted("https://example.com", None)),
        ];
        let same_post_id_on_another_site = insert(
            &mut conn,
            &self_hosted("https://other.example.com", editing_post_5),
        );

        let duplicate =
            Err(LocalContentError::RemotePostAlreadyHasWorkingCopy { post_id: PostId(5) });
        assert_eq!(same_post, duplicate);
        assert_eq!(same_post_differently_written_url, duplicate);
        for result in new_posts {
            assert!(result.is_ok(), "New posts must not collide: {result:?}");
        }
        assert!(
            same_post_id_on_another_site.is_ok(),
            "Another site's post with the same ID is a different post: {same_post_id_on_another_site:?}"
        );
        assert_eq!(count_rows(&conn, SITES), 2);
    }

    /// A site is identified by its WordPress.com site ID once one is given, so
    /// its working copies stay with it when a self-hosted site gets a site ID
    /// (connecting it through Jetpack) and when its URL changes.
    #[test]
    fn test_a_site_keeps_its_working_copies_when_it_gains_a_site_id_or_changes_url() {
        let mut conn = migrated_connection();
        let url_only = site("https://example.com", None);
        let connected = site("https://example.com", Some(5));
        let moved = site("https://new-domain.example", Some(5));
        let ids_for = |conn: &Connection, site: &SiteIdentity| -> Vec<WorkingCopyId> {
            WorkingCopyRepository
                .select_posts_by_site(conn, site)
                .expect("Select should succeed")
                .into_iter()
                .map(|post| post.id)
                .collect()
        };

        let draft = insert_post(&mut conn, &simple_post("Draft"), &new_post_on(&url_only));
        let found_once_connected = ids_for(&conn, &connected);
        insert_post(
            &mut conn,
            &simple_post("After connecting"),
            &new_post_on(&connected),
        );
        insert_post(
            &mut conn,
            &simple_post("After moving"),
            &new_post_on(&moved),
        );
        let stored_site = WorkingCopyRepository
            .select_post(&conn, &draft)
            .expect("Select should succeed")
            .expect("The draft should exist")
            .target
            .site;

        assert_eq!(found_once_connected, vec![draft.clone()]);
        assert_eq!(ids_for(&conn, &moved).len(), 3);
        assert_eq!(stored_site, moved, "The site should have its latest URL");
        assert_eq!(count_rows(&conn, SITES), 1);
        // Once the site has a site ID, its old URL alone identifies a site
        // without one, and a different site ID at the URL is a different site.
        assert_eq!(ids_for(&conn, &url_only), vec![]);
        assert_eq!(
            ids_for(&conn, &site("https://example.com", Some(7))),
            vec![]
        );
    }

    #[test]
    fn test_recording_a_push_points_the_working_copy_at_the_pushed_post() {
        let mut conn = migrated_connection();
        let new_post = insert_post(
            &mut conn,
            &simple_post("New"),
            &self_hosted_new_post_target(),
        );
        let editing_99 = insert_post(
            &mut conn,
            &simple_post("Editing 99"),
            &wordpress_com_target_editing(99),
        );
        let before = WorkingCopyRepository
            .select_post(&conn, &new_post)
            .expect("Select should succeed")
            .expect("The new post should exist");
        let pushed_as_7 = RemotePostReference {
            post_id: PostId(7),
            base_modified_gmt: date(1_800_000_000),
        };
        let pushed_99_again = RemotePostReference {
            post_id: PostId(99),
            base_modified_gmt: date(1_800_000_000),
        };

        let tx = conn.transaction().expect("Failed to begin transaction");
        let recorded_new_post = WorkingCopyRepository
            .record_post_push(&tx, &new_post, pushed_as_7)
            .expect("Recording should succeed");
        let recorded_99 = WorkingCopyRepository
            .record_post_push(&tx, &editing_99, pushed_99_again)
            .expect("Recording should succeed");
        tx.commit().expect("Commit should succeed");
        let after = WorkingCopyRepository
            .select_post(&conn, &new_post)
            .expect("Select should succeed")
            .expect("The new post should exist");
        let editing_99_target = WorkingCopyRepository
            .select_post(&conn, &editing_99)
            .expect("Select should succeed")
            .expect("The post should exist")
            .target;

        assert!(recorded_new_post && recorded_99);
        assert_eq!(after.target.remote_post, Some(pushed_as_7));
        assert_eq!(after.target.site, before.target.site);
        assert_eq!(after.working_copy, before.working_copy);
        assert_eq!(after.updated_at, before.updated_at);
        assert_eq!(editing_99_target.remote_post, Some(pushed_99_again));
    }

    #[test]
    fn test_recording_a_push_refuses_what_would_break_the_target() {
        let mut conn = migrated_connection();
        let editing_99 = insert_post(
            &mut conn,
            &simple_post("Editing 99"),
            &wordpress_com_target_editing(99),
        );
        let new_post = insert_post(
            &mut conn,
            &simple_post("New"),
            &new_post_on(&wordpress_com_target_editing(99).site),
        );
        let page = insert_page(
            &mut conn,
            &PageWorkingCopy {
                common: common("A page"),
                parent: None,
            },
            &self_hosted_new_post_target(),
        );
        let pushed_as = |post_id: i64| RemotePostReference {
            post_id: PostId(post_id),
            base_modified_gmt: date(1_800_000_000),
        };

        let tx = conn.transaction().expect("Failed to begin transaction");
        let to_another_post =
            WorkingCopyRepository.record_post_push(&tx, &editing_99, pushed_as(5));
        let to_a_post_with_a_working_copy =
            WorkingCopyRepository.record_post_push(&tx, &new_post, pushed_as(99));
        let page_as_a_post = WorkingCopyRepository.record_post_push(&tx, &page, pushed_as(5));
        tx.commit().expect("Commit should succeed");
        let new_post_target = WorkingCopyRepository
            .select_post(&conn, &new_post)
            .expect("Select should succeed")
            .expect("The new post should exist")
            .target;

        assert_eq!(
            to_another_post,
            Err(LocalContentError::PushedToAnotherRemotePost {
                edited_post_id: PostId(99),
                pushed_post_id: PostId(5),
            })
        );
        assert_eq!(
            to_a_post_with_a_working_copy,
            Err(LocalContentError::RemotePostAlreadyHasWorkingCopy {
                post_id: PostId(99)
            })
        );
        assert_eq!(new_post_target.remote_post, None);
        assert_eq!(page_as_a_post, Ok(false));
    }

    #[test]
    fn test_deleting_a_site_deletes_its_working_copies_and_nothing_else() {
        let mut conn = migrated_connection();
        let removed = site("https://removed.example", Some(5));
        let kept = site("https://kept.example", None);
        insert_post(
            &mut conn,
            &simple_post("Removed post"),
            &new_post_on(&removed),
        );
        insert_page(
            &mut conn,
            &PageWorkingCopy {
                common: common("Removed page"),
                parent: None,
            },
            &new_post_on(&removed),
        );
        let kept_post = insert_post(&mut conn, &simple_post("Kept"), &new_post_on(&kept));

        let tx = conn.transaction().expect("Failed to begin transaction");
        let deleted = WorkingCopyRepository
            .delete_by_site(&tx, &removed)
            .expect("Delete should succeed");
        let deleted_again = WorkingCopyRepository
            .delete_by_site(&tx, &removed)
            .expect("Delete should succeed");
        tx.commit().expect("Commit should succeed");
        let remaining = WorkingCopyRepository
            .select_all_posts(&conn)
            .expect("Select should succeed");

        assert_eq!((deleted, deleted_again), (2, 0));
        assert_eq!(
            remaining
                .into_iter()
                .map(|post| post.id)
                .collect::<Vec<_>>(),
            vec![kept_post]
        );
        assert_eq!(count_rows(&conn, PAGE_FIELDS), 0);
        assert_eq!(count_rows(&conn, SITES), 1);
    }
}
