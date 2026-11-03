use rusqlite::types::{FromSqlError, Type};
use rusqlite::{OptionalExtension, Row};
use wp_api::{
    posts::{PostCommentStatus, PostFormat, PostId, PostPingStatus},
    users::UserId,
};
use wp_mobile_sqlite::{ColumnIndex, RowExt};

use crate::{
    LocalContentError,
    db_types::{
        working_copy::{
            FeaturedImage, PageFieldsColumn, PageWorkingCopy, PageWorkingCopyFields,
            PostFieldsColumn, PostWorkingCopy, PostWorkingCopyFields, PublishDate, StatusSetting,
            WorkingCopyColumn, WorkingCopyCommonFields, WorkingCopyId, WorkingCopyKind,
            categories_from_json, categories_to_json, tags_from_json, tags_to_json,
        },
        working_copy_remote_target::{RemotePostReference, SiteIdentity, WorkingCopyRemoteTarget},
    },
    repository::{InTransaction, QueryExecutor},
};
use remote_targets::{
    insert_remote_target, record_push, select_id_by_remote_post, select_remote_target,
};
use sites::select_site_rowid;

mod remote_targets;
mod sites;

const WORKING_COPIES: &str = "working_copies";
const POST_FIELDS: &str = "post_working_copy_fields";
const PAGE_FIELDS: &str = "page_working_copy_fields";
const REMOTE_TARGETS: &str = "working_copy_remote_targets";
const SITES: &str = "sites";

/// Repository for working copies and their remote targets.
///
/// A working copy spans its row in `working_copies`, the fields table for its
/// kind, and its row in `working_copy_remote_targets`, which references a row in
/// `sites`. Writes that touch more than one of them require a transaction, so a
/// working copy is never stored without its fields or its remote target.
///
/// Inserting a working copy adds its site to `sites` if no row matches it. If
/// a row does, the row takes the site's URL, and its WordPress.com site ID if
/// the row had none.
pub struct WorkingCopyRepository;

impl WorkingCopyRepository {
    /// Insert a post working copy with its fields and remote target, and return
    /// its newly generated ID.
    ///
    /// Fails with `RemotePostAlreadyHasWorkingCopy` if the remote target's
    /// remote post already has a working copy. Roll `transaction` back after
    /// any error, since the rows written before it stay in the transaction.
    pub fn insert_post(
        &self,
        transaction: &impl InTransaction,
        fields: &PostWorkingCopyFields,
        remote_target: &WorkingCopyRemoteTarget,
    ) -> Result<WorkingCopyId, LocalContentError> {
        let (id, rowid) = insert_common(transaction, WorkingCopyKind::Post, &fields.common)?;
        transaction.execute(
            &format!(
                "INSERT INTO {POST_FIELDS} (working_copy_rowid, category_ids, tags, format, sticky)
                 VALUES (?, ?, ?, ?, ?)"
            ),
            rusqlite::params![
                rowid,
                categories_to_json(&fields.categories)?,
                tags_to_json(&fields.tags)?,
                fields.format.as_str(),
                fields.sticky,
            ],
        )?;
        insert_remote_target(transaction, rowid, remote_target)?;
        Ok(id)
    }

    /// Insert a page working copy with its fields and remote target, and return
    /// its newly generated ID.
    ///
    /// Fails with `RemotePostAlreadyHasWorkingCopy` if the remote target's
    /// remote post already has a working copy. Roll `transaction` back after
    /// any error, since the rows written before it stay in the transaction.
    pub fn insert_page(
        &self,
        transaction: &impl InTransaction,
        fields: &PageWorkingCopyFields,
        remote_target: &WorkingCopyRemoteTarget,
    ) -> Result<WorkingCopyId, LocalContentError> {
        let (id, rowid) = insert_common(transaction, WorkingCopyKind::Page, &fields.common)?;
        transaction.execute(
            &format!("INSERT INTO {PAGE_FIELDS} (working_copy_rowid, parent_id) VALUES (?, ?)"),
            rusqlite::params![rowid, fields.parent.map(|parent| parent.0)],
        )?;
        insert_remote_target(transaction, rowid, remote_target)?;
        Ok(id)
    }

    /// Replace the fields of a post working copy, and set its `updated_at` to
    /// now.
    ///
    /// Returns `false` if no post working copy has the ID.
    pub fn update_post(
        &self,
        transaction: &impl InTransaction,
        id: &WorkingCopyId,
        fields: &PostWorkingCopyFields,
    ) -> Result<bool, LocalContentError> {
        let Some(rowid) = update_common(transaction, WorkingCopyKind::Post, id, &fields.common)?
        else {
            return Ok(false);
        };
        transaction.execute(
            &format!(
                "UPDATE {POST_FIELDS} SET category_ids = ?, tags = ?, format = ?, sticky = ?
                 WHERE working_copy_rowid = ?"
            ),
            rusqlite::params![
                categories_to_json(&fields.categories)?,
                tags_to_json(&fields.tags)?,
                fields.format.as_str(),
                fields.sticky,
                rowid,
            ],
        )?;
        Ok(true)
    }

    /// Replace the fields of a page working copy, and set its `updated_at` to
    /// now.
    ///
    /// Returns `false` if no page working copy has the ID.
    pub fn update_page(
        &self,
        transaction: &impl InTransaction,
        id: &WorkingCopyId,
        fields: &PageWorkingCopyFields,
    ) -> Result<bool, LocalContentError> {
        let Some(rowid) = update_common(transaction, WorkingCopyKind::Page, id, &fields.common)?
        else {
            return Ok(false);
        };
        transaction.execute(
            &format!("UPDATE {PAGE_FIELDS} SET parent_id = ? WHERE working_copy_rowid = ?"),
            rusqlite::params![fields.parent.map(|parent| parent.0), rowid],
        )?;
        Ok(true)
    }

    /// Record that a post working copy was pushed, by setting its remote post
    /// to `remote_post`. Its fields and `updated_at` stay as they are.
    ///
    /// Returns `false` if no post working copy has the ID. Fails with
    /// `RemotePostAlreadyHasWorkingCopy` if another working copy has that
    /// remote post, and with `PushedToAnotherRemotePost` if this one already
    /// has a different remote post.
    pub fn record_post_push(
        &self,
        transaction: &impl InTransaction,
        id: &WorkingCopyId,
        remote_post: RemotePostReference,
    ) -> Result<bool, LocalContentError> {
        record_push(transaction, WorkingCopyKind::Post, id, remote_post)
    }

    /// Record that a page working copy was pushed, by setting its remote post
    /// to `remote_post`. Its fields and `updated_at` stay as they are.
    ///
    /// Returns `false` if no page working copy has the ID. Fails with
    /// `RemotePostAlreadyHasWorkingCopy` if another working copy has that
    /// remote post, and with `PushedToAnotherRemotePost` if this one already
    /// has a different remote post.
    pub fn record_page_push(
        &self,
        transaction: &impl InTransaction,
        id: &WorkingCopyId,
        remote_post: RemotePostReference,
    ) -> Result<bool, LocalContentError> {
        record_push(transaction, WorkingCopyKind::Page, id, remote_post)
    }

    /// Delete a post working copy. Its fields and remote target are deleted with it.
    ///
    /// Returns `false` if no post working copy has the ID.
    pub fn delete_post(
        &self,
        executor: &impl QueryExecutor,
        id: &WorkingCopyId,
    ) -> Result<bool, LocalContentError> {
        delete_working_copy(executor, WorkingCopyKind::Post, id)
    }

    /// Delete a page working copy. Its fields and remote target are deleted with it.
    ///
    /// Returns `false` if no page working copy has the ID.
    pub fn delete_page(
        &self,
        executor: &impl QueryExecutor,
        id: &WorkingCopyId,
    ) -> Result<bool, LocalContentError> {
        delete_working_copy(executor, WorkingCopyKind::Page, id)
    }

    /// Delete every post and page working copy for `site`, and the site's row.
    /// Their fields and remote targets are deleted with them.
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
                 WHERE rowid IN (SELECT working_copy_rowid FROM {REMOTE_TARGETS} WHERE site_rowid = ?)"
            ),
            [site_rowid],
        )?;
        transaction.execute(
            &format!("DELETE FROM {SITES} WHERE rowid = ?"),
            [site_rowid],
        )?;
        Ok(deleted)
    }

    /// Select a post working copy by its ID.
    ///
    /// Returns `Ok(None)` if no post working copy has the ID, including when
    /// the ID is a page's.
    pub fn select_post(
        &self,
        executor: &impl QueryExecutor,
        id: &WorkingCopyId,
    ) -> Result<Option<PostWorkingCopy>, LocalContentError> {
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
                    .map_err(|e| conversion_error(CategoryIds, Type::Text, e))?,
                tags_from_json(&tags_json).map_err(|e| conversion_error(Tags, Type::Text, e))?,
                format,
                row.get_column::<bool, _>(Sticky)?,
            ))
        })?;
        let remote_target = select_remote_target(executor, common_row.rowid)?;

        Ok(Some(PostWorkingCopy {
            id: common_row.id,
            fields: PostWorkingCopyFields {
                common: common_row.common,
                categories,
                tags,
                format: format
                    .parse::<PostFormat>()
                    .unwrap_or(PostFormat::Custom(format)),
                sticky,
            },
            remote_target,
            created_at: common_row.created_at,
            updated_at: common_row.updated_at,
        }))
    }

    /// Select a page working copy by its ID.
    ///
    /// Returns `Ok(None)` if no page working copy has the ID, including when
    /// the ID is a post's.
    pub fn select_page(
        &self,
        executor: &impl QueryExecutor,
        id: &WorkingCopyId,
    ) -> Result<Option<PageWorkingCopy>, LocalContentError> {
        let Some(common_row) = select_common(executor, WorkingCopyKind::Page, id)? else {
            return Ok(None);
        };
        let mut stmt = executor.prepare(&format!(
            "SELECT * FROM {PAGE_FIELDS} WHERE working_copy_rowid = ?"
        ))?;
        let parent: Option<i64> = stmt.query_row([common_row.rowid], |row| {
            row.get_column(PageFieldsColumn::ParentId)
        })?;
        let remote_target = select_remote_target(executor, common_row.rowid)?;

        Ok(Some(PageWorkingCopy {
            id: common_row.id,
            fields: PageWorkingCopyFields {
                common: common_row.common,
                parent: parent.map(PostId),
            },
            remote_target,
            created_at: common_row.created_at,
            updated_at: common_row.updated_at,
        }))
    }

    /// Select the post working copy whose remote post is `post_id` on `site`.
    ///
    /// Returns `Ok(None)` if there is none.
    pub fn select_post_by_remote_post(
        &self,
        executor: &impl QueryExecutor,
        site: &SiteIdentity,
        post_id: PostId,
    ) -> Result<Option<PostWorkingCopy>, LocalContentError> {
        match select_id_by_remote_post(executor, WorkingCopyKind::Post, site, post_id)? {
            Some(id) => self.select_post(executor, &id),
            None => Ok(None),
        }
    }

    /// Select the page working copy whose remote post is `post_id` on `site`.
    ///
    /// Returns `Ok(None)` if there is none.
    pub fn select_page_by_remote_post(
        &self,
        executor: &impl QueryExecutor,
        site: &SiteIdentity,
        post_id: PostId,
    ) -> Result<Option<PageWorkingCopy>, LocalContentError> {
        match select_id_by_remote_post(executor, WorkingCopyKind::Page, site, post_id)? {
            Some(id) => self.select_page(executor, &id),
            None => Ok(None),
        }
    }

    /// Select every post working copy for `site`, newest `updated_at` first.
    pub fn select_posts_by_site(
        &self,
        executor: &impl QueryExecutor,
        site: &SiteIdentity,
    ) -> Result<Vec<PostWorkingCopy>, LocalContentError> {
        select_ids_by_site(executor, WorkingCopyKind::Post, site)?
            .iter()
            .filter_map(|id| self.select_post(executor, id).transpose())
            .collect()
    }

    /// Select every page working copy for `site`, newest `updated_at` first.
    pub fn select_pages_by_site(
        &self,
        executor: &impl QueryExecutor,
        site: &SiteIdentity,
    ) -> Result<Vec<PageWorkingCopy>, LocalContentError> {
        select_ids_by_site(executor, WorkingCopyKind::Page, site)?
            .iter()
            .filter_map(|id| self.select_page(executor, id).transpose())
            .collect()
    }

    /// Select every post working copy on every site, newest `updated_at` first.
    pub fn select_all_posts(
        &self,
        executor: &impl QueryExecutor,
    ) -> Result<Vec<PostWorkingCopy>, LocalContentError> {
        select_all_ids(executor, WorkingCopyKind::Post)?
            .iter()
            .filter_map(|id| self.select_post(executor, id).transpose())
            .collect()
    }

    /// Select every page working copy on every site, newest `updated_at` first.
    pub fn select_all_pages(
        &self,
        executor: &impl QueryExecutor,
    ) -> Result<Vec<PageWorkingCopy>, LocalContentError> {
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
    common: WorkingCopyCommonFields,
    created_at: String,
    updated_at: String,
}

fn insert_common(
    transaction: &impl InTransaction,
    kind: WorkingCopyKind,
    common: &WorkingCopyCommonFields,
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
            common.comment_status.as_ref().map(ToString::to_string),
            common.ping_status.as_ref().map(ToString::to_string),
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
    common: &WorkingCopyCommonFields,
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
                common.comment_status.as_ref().map(ToString::to_string),
                common.ping_status.as_ref().map(ToString::to_string),
                id,
                kind.as_str(),
            ],
            |row| row.get(0),
        )
        .optional()?;
    Ok(rowid)
}

/// The fields and remote target rows go with the working copy through `ON DELETE CASCADE`.
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
         JOIN {REMOTE_TARGETS} t ON t.working_copy_rowid = w.rowid
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
        common: WorkingCopyCommonFields {
            title: row.get_column(C::Title)?,
            content: row.get_column(C::Content)?,
            excerpt: row.get_column(C::Excerpt)?,
            slug: row.get_column(C::Slug)?,
            status: StatusSetting::from_columns(&status, password)
                .map_err(|e| conversion_error(C::Status, Type::Text, e))?,
            publish_date: PublishDate::from_column(publish_date)
                .map_err(|e| conversion_error(C::PublishDate, Type::Integer, e))?,
            author: author.map(UserId),
            featured_image: FeaturedImage::from_column(featured_media_id),
            comment_status: comment_status
                .map(|status| status.parse().unwrap_or(PostCommentStatus::Custom(status))),
            ping_status: ping_status
                .map(|status| status.parse().unwrap_or(PostPingStatus::Custom(status))),
        },
        created_at: row.get_column(C::CreatedAt)?,
        updated_at: row.get_column(C::UpdatedAt)?,
    })
}

fn conversion_error(
    column: impl ColumnIndex,
    column_type: Type,
    err: FromSqlError,
) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(column.as_index(), column_type, Box::new(err))
}

#[cfg(test)]
mod tests;
