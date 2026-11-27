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
        editor_session::EditorSessionId,
        working_copy::{
            FeaturedImage, FieldSetColumn, PageFieldsColumn, PageWorkingCopy,
            PageWorkingCopyFields, PostFieldsColumn, PostWorkingCopy, PostWorkingCopyFields,
            PublishDate, StatusSetting, WorkingCopyCommonFields, WorkingCopyId, WorkingCopyKind,
            additional_fields_from_json, additional_fields_to_json, meta_from_json, meta_to_json,
            term_ids_from_json, term_ids_to_json,
        },
        working_copy_remote_target::{RemotePostReference, SiteIdentity, WorkingCopyRemoteTarget},
        working_copy_version::{WorkingCopyVersion, WorkingCopyVersionId},
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
const CONTENTS: &str = "working_copy_contents";
const FIELD_SETS: &str = "working_copy_field_sets";
const POST_FIELDS: &str = "post_field_sets";
const PAGE_FIELDS: &str = "page_field_sets";
const VERSIONS: &str = "working_copy_versions";
const REMOTE_TARGETS: &str = "working_copy_remote_targets";
const SITES: &str = "sites";
const EDITOR_SESSIONS: &str = "editor_sessions";

/// Repository for working copies, their history and their remote targets.
///
/// A working copy is a row in `working_copies`, its versions, and its row in
/// `working_copy_remote_targets`, which references a row in `sites`. Each
/// version refers to a row in `working_copy_contents` and one in
/// `working_copy_field_sets`, with that row's match in the field sets table
/// for its kind. The newest version holds the working copy's fields. Writes
/// that touch more than one table require a transaction, so a working copy is
/// never stored without a version or its remote target.
///
/// Inserting a working copy adds its site to `sites` if no row matches it. If
/// a row does, the row takes the site's URL, and its WordPress.com site ID if
/// the row had none.
pub struct WorkingCopyRepository;

impl WorkingCopyRepository {
    /// Insert a post working copy with its first version and its remote
    /// target, and return its newly generated ID.
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
        let (id, rowid) = insert_working_copy(transaction, WorkingCopyKind::Post)?;
        append_version(
            transaction,
            rowid,
            None,
            Changes::ALL,
            &fields.common.content,
            || insert_post_field_set(transaction, rowid, fields),
            None,
        )?;
        insert_remote_target(transaction, rowid, remote_target)?;
        Ok(id)
    }

    /// Insert a page working copy with its first version and its remote
    /// target, and return its newly generated ID.
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
        let (id, rowid) = insert_working_copy(transaction, WorkingCopyKind::Page)?;
        append_version(
            transaction,
            rowid,
            None,
            Changes::ALL,
            &fields.common.content,
            || insert_page_field_set(transaction, rowid, fields),
            None,
        )?;
        insert_remote_target(transaction, rowid, remote_target)?;
        Ok(id)
    }

    /// Save `fields` as a post working copy's fields, by adding a version whose
    /// parent is its newest one. Nothing is added if `fields` are the newest
    /// version's.
    ///
    /// The version refers to its parent's content row if the content didn't
    /// change, and to its parent's field set row if nothing else did.
    /// `editor_session` is the session the change was recorded in, if any.
    ///
    /// Returns `false` if no post working copy has the ID. Fails with
    /// `EditorSessionNotFound` if no session has `editor_session`.
    pub fn update_post(
        &self,
        transaction: &impl InTransaction,
        id: &WorkingCopyId,
        fields: &PostWorkingCopyFields,
        editor_session: Option<&EditorSessionId>,
    ) -> Result<bool, LocalContentError> {
        let Some(row) = select_working_copy_row(transaction, WorkingCopyKind::Post, id)? else {
            return Ok(false);
        };
        let session_rowid = editor_session
            .map(|session| select_session_rowid(transaction, session))
            .transpose()?;
        let current = select_newest_version(transaction, row.rowid)?;
        let current_fields = select_post_fields(transaction, &current)?;
        let changes = Changes {
            content: fields.common.content != current_fields.common.content,
            other_fields: {
                let mut other_fields = fields.clone();
                other_fields.common.content = current_fields.common.content.clone();
                other_fields != current_fields
            },
        };
        append_version(
            transaction,
            row.rowid,
            Some(&current),
            changes,
            &fields.common.content,
            || insert_post_field_set(transaction, row.rowid, fields),
            session_rowid,
        )?;
        Ok(true)
    }

    /// Save `fields` as a page working copy's fields, by adding a version whose
    /// parent is its newest one. Nothing is added if `fields` are the newest
    /// version's.
    ///
    /// The version refers to its parent's content row if the content didn't
    /// change, and to its parent's field set row if nothing else did.
    /// `editor_session` is the session the change was recorded in, if any.
    ///
    /// Returns `false` if no page working copy has the ID. Fails with
    /// `EditorSessionNotFound` if no session has `editor_session`.
    pub fn update_page(
        &self,
        transaction: &impl InTransaction,
        id: &WorkingCopyId,
        fields: &PageWorkingCopyFields,
        editor_session: Option<&EditorSessionId>,
    ) -> Result<bool, LocalContentError> {
        let Some(row) = select_working_copy_row(transaction, WorkingCopyKind::Page, id)? else {
            return Ok(false);
        };
        let session_rowid = editor_session
            .map(|session| select_session_rowid(transaction, session))
            .transpose()?;
        let current = select_newest_version(transaction, row.rowid)?;
        let current_fields = select_page_fields(transaction, &current)?;
        let changes = Changes {
            content: fields.common.content != current_fields.common.content,
            other_fields: {
                let mut other_fields = fields.clone();
                other_fields.common.content = current_fields.common.content.clone();
                other_fields != current_fields
            },
        };
        append_version(
            transaction,
            row.rowid,
            Some(&current),
            changes,
            &fields.common.content,
            || insert_page_field_set(transaction, row.rowid, fields),
            session_rowid,
        )?;
        Ok(true)
    }

    /// Record that a post working copy was pushed, by setting its remote post
    /// to `remote_post`. Its versions stay as they are.
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
    /// to `remote_post`. Its versions stay as they are.
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

    /// Delete a post working copy. Its versions and remote target are deleted
    /// with it.
    ///
    /// Returns `false` if no post working copy has the ID.
    pub fn delete_post(
        &self,
        executor: &impl QueryExecutor,
        id: &WorkingCopyId,
    ) -> Result<bool, LocalContentError> {
        delete_working_copy(executor, WorkingCopyKind::Post, id)
    }

    /// Delete a page working copy. Its versions and remote target are deleted
    /// with it.
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
    /// Their versions and remote targets are deleted with them.
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

    /// Select a post working copy by its ID, with the fields of its newest
    /// version.
    ///
    /// Returns `Ok(None)` if no post working copy has the ID, including when
    /// the ID is a page's.
    pub fn select_post(
        &self,
        executor: &impl QueryExecutor,
        id: &WorkingCopyId,
    ) -> Result<Option<PostWorkingCopy>, LocalContentError> {
        let Some(row) = select_working_copy_row(executor, WorkingCopyKind::Post, id)? else {
            return Ok(None);
        };
        let newest = select_newest_version(executor, row.rowid)?;
        Ok(Some(PostWorkingCopy {
            fields: select_post_fields(executor, &newest)?,
            remote_target: select_remote_target(executor, row.rowid)?,
            id: row.id,
            created_at: row.created_at,
            updated_at: newest.created_at,
        }))
    }

    /// Select a page working copy by its ID, with the fields of its newest
    /// version.
    ///
    /// Returns `Ok(None)` if no page working copy has the ID, including when
    /// the ID is a post's.
    pub fn select_page(
        &self,
        executor: &impl QueryExecutor,
        id: &WorkingCopyId,
    ) -> Result<Option<PageWorkingCopy>, LocalContentError> {
        let Some(row) = select_working_copy_row(executor, WorkingCopyKind::Page, id)? else {
            return Ok(None);
        };
        let newest = select_newest_version(executor, row.rowid)?;
        Ok(Some(PageWorkingCopy {
            fields: select_page_fields(executor, &newest)?,
            remote_target: select_remote_target(executor, row.rowid)?,
            id: row.id,
            created_at: row.created_at,
            updated_at: newest.created_at,
        }))
    }

    /// Select the versions of a post or page working copy, oldest first.
    ///
    /// Returns `Ok(None)` if no working copy has the ID.
    pub fn select_versions(
        &self,
        executor: &impl QueryExecutor,
        id: &WorkingCopyId,
    ) -> Result<Option<Vec<WorkingCopyVersion>>, LocalContentError> {
        let exists = executor
            .prepare(&format!("SELECT 1 FROM {WORKING_COPIES} WHERE uuid = ?"))?
            .query_row([id], |_| Ok(()))
            .optional()?
            .is_some();
        if !exists {
            return Ok(None);
        }
        let mut stmt = executor.prepare(&format!(
            "SELECT v.uuid, p.uuid, s.uuid, v.created_at FROM {VERSIONS} v
             JOIN {WORKING_COPIES} w ON w.rowid = v.working_copy_rowid
             LEFT JOIN {VERSIONS} p ON p.rowid = v.parent_rowid
             LEFT JOIN {EDITOR_SESSIONS} s ON s.rowid = v.editor_session_rowid
             WHERE w.uuid = ?
             ORDER BY v.rowid"
        ))?;
        let versions = stmt
            .query_map([id], |row| {
                Ok(WorkingCopyVersion {
                    id: row.get(0)?,
                    parent: row.get(1)?,
                    editor_session_id: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(versions))
    }

    /// Select the fields of one version of a post working copy.
    ///
    /// Returns `Ok(None)` if no version of a post working copy has the ID.
    pub fn select_post_version(
        &self,
        executor: &impl QueryExecutor,
        id: &WorkingCopyVersionId,
    ) -> Result<Option<PostWorkingCopyFields>, LocalContentError> {
        select_version(executor, WorkingCopyKind::Post, id)?
            .map(|version| select_post_fields(executor, &version))
            .transpose()
    }

    /// Select the fields of one version of a page working copy.
    ///
    /// Returns `Ok(None)` if no version of a page working copy has the ID.
    pub fn select_page_version(
        &self,
        executor: &impl QueryExecutor,
        id: &WorkingCopyVersionId,
    ) -> Result<Option<PageWorkingCopyFields>, LocalContentError> {
        select_version(executor, WorkingCopyKind::Page, id)?
            .map(|version| select_page_fields(executor, &version))
            .transpose()
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

    /// Select every post working copy for `site`, most recently updated first.
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

    /// Select every page working copy for `site`, most recently updated first.
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

    /// Select every post working copy on every site, most recently updated
    /// first.
    pub fn select_all_posts(
        &self,
        executor: &impl QueryExecutor,
    ) -> Result<Vec<PostWorkingCopy>, LocalContentError> {
        select_all_ids(executor, WorkingCopyKind::Post)?
            .iter()
            .filter_map(|id| self.select_post(executor, id).transpose())
            .collect()
    }

    /// Select every page working copy on every site, most recently updated
    /// first.
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

/// The most recently updated working copy first: version rowids only grow, so
/// the working copy with the highest one was saved last.
const NEWEST_VERSION_FIRST: &str = "(SELECT MAX(v.rowid) FROM working_copy_versions v
      WHERE v.working_copy_rowid = w.rowid) DESC";

/// A `working_copies` row.
struct WorkingCopyRow {
    rowid: i64,
    id: WorkingCopyId,
    created_at: String,
}

/// The rows a version refers to.
struct VersionRows {
    rowid: i64,
    content_rowid: i64,
    field_set_rowid: i64,
    created_at: String,
}

/// What a save changes, compared with the version it's made from.
struct Changes {
    content: bool,
    other_fields: bool,
}

impl Changes {
    const ALL: Self = Self {
        content: true,
        other_fields: true,
    };
}

fn insert_working_copy(
    transaction: &impl InTransaction,
    kind: WorkingCopyKind,
) -> Result<(WorkingCopyId, i64), LocalContentError> {
    let id = WorkingCopyId::generate();
    let mut stmt = transaction.prepare(&format!(
        "INSERT INTO {WORKING_COPIES} (uuid, kind) VALUES (?, ?) RETURNING rowid"
    ))?;
    let rowid = stmt.query_row(rusqlite::params![id, kind.as_str()], |row| row.get(0))?;
    Ok((id, rowid))
}

/// Add a version to the working copy at `working_copy_rowid`, made from
/// `parent`. Only what `changes` says changed gets new rows: the content from
/// `content`, and the other fields from `insert_field_set`, which returns the
/// new field set's rowid. Nothing is added if nothing changed.
///
/// A first version takes its working copy's `created_at`, so a working copy
/// that was never saved again was last updated when it was created.
fn append_version(
    transaction: &impl InTransaction,
    working_copy_rowid: i64,
    parent: Option<&VersionRows>,
    changes: Changes,
    content: &str,
    insert_field_set: impl FnOnce() -> Result<i64, LocalContentError>,
    editor_session_rowid: Option<i64>,
) -> Result<(), LocalContentError> {
    let (content_rowid, field_set_rowid) = match parent {
        Some(_) if !changes.content && !changes.other_fields => return Ok(()),
        Some(parent) => (
            if changes.content {
                insert_content(transaction, working_copy_rowid, content)?
            } else {
                parent.content_rowid
            },
            if changes.other_fields {
                insert_field_set()?
            } else {
                parent.field_set_rowid
            },
        ),
        None => (
            insert_content(transaction, working_copy_rowid, content)?,
            insert_field_set()?,
        ),
    };
    transaction.execute(
        &format!(
            "INSERT INTO {VERSIONS} (
                 uuid, working_copy_rowid, parent_rowid, content_rowid, field_set_rowid,
                 editor_session_rowid, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, CASE
                 WHEN ?3 IS NULL THEN (SELECT created_at FROM {WORKING_COPIES} WHERE rowid = ?2)
                 ELSE strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             END)"
        ),
        rusqlite::params![
            WorkingCopyVersionId::generate(),
            working_copy_rowid,
            parent.map(|parent| parent.rowid),
            content_rowid,
            field_set_rowid,
            editor_session_rowid,
        ],
    )?;
    Ok(())
}

fn insert_content(
    transaction: &impl InTransaction,
    working_copy_rowid: i64,
    content: &str,
) -> Result<i64, LocalContentError> {
    let mut stmt = transaction.prepare(&format!(
        "INSERT INTO {CONTENTS} (working_copy_rowid, content) VALUES (?, ?) RETURNING rowid"
    ))?;
    Ok(
        stmt.query_row(rusqlite::params![working_copy_rowid, content], |row| {
            row.get(0)
        })?,
    )
}

/// Insert the fields every kind has, other than the content, and return the
/// field set's rowid.
fn insert_common_field_set(
    transaction: &impl InTransaction,
    working_copy_rowid: i64,
    common: &WorkingCopyCommonFields,
) -> Result<i64, LocalContentError> {
    let (status, password) = common.status.to_columns();
    let mut stmt = transaction.prepare(&format!(
        "INSERT INTO {FIELD_SETS} (
             working_copy_rowid, title, excerpt, slug, status, password, publish_date,
             author_id, featured_media_id, comment_status, ping_status, template, menu_order
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING rowid"
    ))?;
    Ok(stmt.query_row(
        rusqlite::params![
            working_copy_rowid,
            common.title,
            common.excerpt,
            common.slug,
            status,
            password,
            common.publish_date.to_column(),
            common.author.map(|author| author.0),
            common.featured_image.to_column(),
            common.comment_status.as_ref().map(ToString::to_string),
            common.ping_status.as_ref().map(ToString::to_string),
            common.template,
            common.menu_order,
        ],
        |row| row.get(0),
    )?)
}

fn insert_post_field_set(
    transaction: &impl InTransaction,
    working_copy_rowid: i64,
    fields: &PostWorkingCopyFields,
) -> Result<i64, LocalContentError> {
    let rowid = insert_common_field_set(transaction, working_copy_rowid, &fields.common)?;
    transaction.execute(
        &format!(
            "INSERT INTO {POST_FIELDS} (
                 field_set_rowid, category_ids, tag_ids, format, sticky, meta, additional_fields
             ) VALUES (?, ?, ?, ?, ?, ?, ?)"
        ),
        rusqlite::params![
            rowid,
            term_ids_to_json(&fields.categories)?,
            term_ids_to_json(&fields.tags)?,
            fields.format.as_str(),
            fields.sticky,
            meta_to_json(&fields.meta)?,
            additional_fields_to_json(&fields.additional_fields)?,
        ],
    )?;
    Ok(rowid)
}

fn insert_page_field_set(
    transaction: &impl InTransaction,
    working_copy_rowid: i64,
    fields: &PageWorkingCopyFields,
) -> Result<i64, LocalContentError> {
    let rowid = insert_common_field_set(transaction, working_copy_rowid, &fields.common)?;
    transaction.execute(
        &format!("INSERT INTO {PAGE_FIELDS} (field_set_rowid, parent_id) VALUES (?, ?)"),
        rusqlite::params![rowid, fields.parent.map(|parent| parent.0)],
    )?;
    Ok(rowid)
}

fn select_working_copy_row(
    executor: &impl QueryExecutor,
    kind: WorkingCopyKind,
    id: &WorkingCopyId,
) -> Result<Option<WorkingCopyRow>, LocalContentError> {
    let mut stmt = executor.prepare(&format!(
        "SELECT rowid, uuid, created_at FROM {WORKING_COPIES} WHERE uuid = ? AND kind = ?"
    ))?;
    Ok(stmt
        .query_row(rusqlite::params![id, kind.as_str()], |row| {
            Ok(WorkingCopyRow {
                rowid: row.get(0)?,
                id: row.get(1)?,
                created_at: row.get(2)?,
            })
        })
        .optional()?)
}

/// The newest version of the working copy at `working_copy_rowid`. Every
/// working copy has at least one.
fn select_newest_version(
    executor: &impl QueryExecutor,
    working_copy_rowid: i64,
) -> Result<VersionRows, LocalContentError> {
    let mut stmt = executor.prepare(&format!(
        "SELECT rowid, content_rowid, field_set_rowid, created_at FROM {VERSIONS}
         WHERE working_copy_rowid = ? ORDER BY rowid DESC LIMIT 1"
    ))?;
    Ok(stmt.query_row([working_copy_rowid], version_rows_from_row)?)
}

fn select_version(
    executor: &impl QueryExecutor,
    kind: WorkingCopyKind,
    id: &WorkingCopyVersionId,
) -> Result<Option<VersionRows>, LocalContentError> {
    let mut stmt = executor.prepare(&format!(
        "SELECT v.rowid, v.content_rowid, v.field_set_rowid, v.created_at FROM {VERSIONS} v
         JOIN {WORKING_COPIES} w ON w.rowid = v.working_copy_rowid
         WHERE v.uuid = ? AND w.kind = ?"
    ))?;
    Ok(stmt
        .query_row(rusqlite::params![id, kind.as_str()], version_rows_from_row)
        .optional()?)
}

fn version_rows_from_row(row: &Row) -> rusqlite::Result<VersionRows> {
    Ok(VersionRows {
        rowid: row.get(0)?,
        content_rowid: row.get(1)?,
        field_set_rowid: row.get(2)?,
        created_at: row.get(3)?,
    })
}

fn select_session_rowid(
    executor: &impl QueryExecutor,
    id: &EditorSessionId,
) -> Result<i64, LocalContentError> {
    executor
        .prepare(&format!(
            "SELECT rowid FROM {EDITOR_SESSIONS} WHERE uuid = ?"
        ))?
        .query_row([id], |row| row.get(0))
        .optional()?
        .ok_or_else(|| LocalContentError::EditorSessionNotFound { id: id.clone() })
}

/// The fields every kind has, from a version's content and field set rows.
fn select_common_fields(
    executor: &impl QueryExecutor,
    version: &VersionRows,
) -> Result<WorkingCopyCommonFields, LocalContentError> {
    let content: String = executor
        .prepare(&format!("SELECT content FROM {CONTENTS} WHERE rowid = ?"))?
        .query_row([version.content_rowid], |row| row.get(0))?;
    let mut stmt = executor.prepare(&format!("SELECT * FROM {FIELD_SETS} WHERE rowid = ?"))?;
    Ok(stmt.query_row([version.field_set_rowid], |row| {
        common_from_row(row, content)
    })?)
}

fn select_post_fields(
    executor: &impl QueryExecutor,
    version: &VersionRows,
) -> Result<PostWorkingCopyFields, LocalContentError> {
    let common = select_common_fields(executor, version)?;
    let mut stmt = executor.prepare(&format!(
        "SELECT * FROM {POST_FIELDS} WHERE field_set_rowid = ?"
    ))?;
    Ok(stmt.query_row([version.field_set_rowid], |row| {
        use PostFieldsColumn::*;
        let categories_json: String = row.get_column(CategoryIds)?;
        let tags_json: String = row.get_column(TagIds)?;
        let format: String = row.get_column(Format)?;
        let meta_json: String = row.get_column(Meta)?;
        let additional_fields_json: String = row.get_column(AdditionalFields)?;
        Ok(PostWorkingCopyFields {
            common,
            categories: term_ids_from_json(&categories_json)
                .map_err(|e| conversion_error(CategoryIds, Type::Text, e))?,
            tags: term_ids_from_json(&tags_json)
                .map_err(|e| conversion_error(TagIds, Type::Text, e))?,
            format: format
                .parse::<PostFormat>()
                .unwrap_or(PostFormat::Custom(format)),
            sticky: row.get_column(Sticky)?,
            meta: meta_from_json(&meta_json).map_err(|e| conversion_error(Meta, Type::Text, e))?,
            additional_fields: additional_fields_from_json(&additional_fields_json)
                .map_err(|e| conversion_error(AdditionalFields, Type::Text, e))?,
        })
    })?)
}

fn select_page_fields(
    executor: &impl QueryExecutor,
    version: &VersionRows,
) -> Result<PageWorkingCopyFields, LocalContentError> {
    let common = select_common_fields(executor, version)?;
    let mut stmt = executor.prepare(&format!(
        "SELECT * FROM {PAGE_FIELDS} WHERE field_set_rowid = ?"
    ))?;
    let parent: Option<i64> = stmt.query_row([version.field_set_rowid], |row| {
        row.get_column(PageFieldsColumn::ParentId)
    })?;
    Ok(PageWorkingCopyFields {
        common,
        parent: parent.map(PostId),
    })
}

/// Everything else of a working copy goes with it through `ON DELETE CASCADE`.
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
         ORDER BY {NEWEST_VERSION_FIRST}"
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
        "SELECT w.uuid FROM {WORKING_COPIES} w WHERE w.kind = ?
         ORDER BY {NEWEST_VERSION_FIRST}"
    ))?;
    let ids = stmt
        .query_map([kind.as_str()], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

fn common_from_row(row: &Row, content: String) -> rusqlite::Result<WorkingCopyCommonFields> {
    use FieldSetColumn as C;

    let status: String = row.get_column(C::Status)?;
    let password: Option<String> = row.get_column(C::Password)?;
    let publish_date: Option<String> = row.get_column(C::PublishDate)?;
    let author: Option<i64> = row.get_column(C::AuthorId)?;
    let featured_media_id: Option<i64> = row.get_column(C::FeaturedMediaId)?;
    let comment_status: Option<String> = row.get_column(C::CommentStatus)?;
    let ping_status: Option<String> = row.get_column(C::PingStatus)?;

    Ok(WorkingCopyCommonFields {
        title: row.get_column(C::Title)?,
        content,
        excerpt: row.get_column(C::Excerpt)?,
        slug: row.get_column(C::Slug)?,
        status: StatusSetting::from_columns(&status, password)
            .map_err(|e| conversion_error(C::Status, Type::Text, e))?,
        publish_date: PublishDate::from_column(publish_date),
        author: author.map(UserId),
        featured_image: FeaturedImage::from_column(featured_media_id),
        comment_status: comment_status
            .map(|status| status.parse().unwrap_or(PostCommentStatus::Custom(status))),
        ping_status: ping_status
            .map(|status| status.parse().unwrap_or(PostPingStatus::Custom(status))),
        template: row.get_column(C::Template)?,
        menu_order: row.get_column(C::MenuOrder)?,
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
