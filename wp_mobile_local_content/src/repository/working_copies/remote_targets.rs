//! Reading and writing `working_copy_remote_targets` rows, including the rule
//! that a remote post has at most one working copy.

use rusqlite::types::{FromSqlError, Type};
use rusqlite::{ErrorCode, OptionalExtension, ffi};
use wp_api::posts::PostId;
use wp_mobile_sqlite::RowExt;

use super::{
    REMOTE_TARGETS, WORKING_COPIES, conversion_error,
    sites::{select_site, select_site_rowid, upsert_site},
};
use crate::{
    LocalContentError,
    db_types::{
        working_copy::{WorkingCopyId, WorkingCopyKind, date_from_timestamp},
        working_copy_remote_target::{
            RemotePostReference, SiteIdentity, WorkingCopyRemoteTarget,
            WorkingCopyRemoteTargetColumn,
        },
    },
    repository::{InTransaction, QueryExecutor},
};

pub(super) fn insert_remote_target(
    transaction: &impl InTransaction,
    rowid: i64,
    remote_target: &WorkingCopyRemoteTarget,
) -> Result<(), LocalContentError> {
    let site_rowid = upsert_site(transaction, &remote_target.site)?;
    let mut stmt = transaction.prepare(&format!(
        "INSERT INTO {REMOTE_TARGETS} (working_copy_rowid, site_rowid, remote_post_id, base_modified_gmt)
         VALUES (?, ?, ?, ?)"
    ))?;
    let result = stmt.execute(rusqlite::params![
        rowid,
        site_rowid,
        remote_target.remote_post.map(|remote| remote.post_id.0),
        remote_target
            .remote_post
            .map(|remote| remote.base_modified_gmt.0.timestamp()),
    ]);
    match remote_target.remote_post {
        Some(remote_post) => remote_post_conflict(result, remote_post.post_id),
        None => {
            result?;
            Ok(())
        }
    }
}

/// Maps the result of writing a remote target whose remote post is `post_id`.
///
/// The only unique constraint a remote target write can break is the one on
/// `(site_rowid, remote_post_id)`, meaning another working copy already has
/// that remote post.
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

pub(super) fn select_remote_target(
    executor: &impl QueryExecutor,
    rowid: i64,
) -> Result<WorkingCopyRemoteTarget, LocalContentError> {
    let mut stmt = executor.prepare(&format!(
        "SELECT * FROM {REMOTE_TARGETS} WHERE working_copy_rowid = ?"
    ))?;
    let (site_rowid, remote_post) = stmt.query_row([rowid], |row| {
        use WorkingCopyRemoteTargetColumn::*;

        let site_rowid: i64 = row.get_column(SiteRowid)?;
        let remote_post_id: Option<i64> = row.get_column(RemotePostId)?;
        let base_modified_gmt: Option<i64> = row.get_column(BaseModifiedGmt)?;
        let remote_post = match (remote_post_id, base_modified_gmt) {
            (Some(post_id), Some(seconds)) => Some(RemotePostReference {
                post_id: PostId(post_id),
                base_modified_gmt: date_from_timestamp(seconds)
                    .map_err(|e| conversion_error(BaseModifiedGmt, Type::Integer, e))?,
            }),
            (None, None) => None,
            _ => {
                return Err(conversion_error(
                    RemotePostId,
                    Type::Integer,
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

    Ok(WorkingCopyRemoteTarget { site, remote_post })
}

pub(super) fn record_push(
    transaction: &impl InTransaction,
    kind: WorkingCopyKind,
    id: &WorkingCopyId,
    remote_post: RemotePostReference,
) -> Result<bool, LocalContentError> {
    let mut stmt = transaction.prepare(&format!(
        "SELECT w.rowid, t.remote_post_id FROM {WORKING_COPIES} w
         JOIN {REMOTE_TARGETS} t ON t.working_copy_rowid = w.rowid
         WHERE w.uuid = ? AND w.kind = ?"
    ))?;
    let Some((rowid, remote_post_id)) = stmt
        .query_row(rusqlite::params![id, kind.as_str()], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?))
        })
        .optional()?
    else {
        return Ok(false);
    };
    if let Some(remote_post_id) = remote_post_id.map(PostId)
        && remote_post_id != remote_post.post_id
    {
        return Err(LocalContentError::PushedToAnotherRemotePost {
            remote_post_id,
            pushed_post_id: remote_post.post_id,
        });
    }

    let mut update = transaction.prepare(&format!(
        "UPDATE {REMOTE_TARGETS} SET remote_post_id = ?, base_modified_gmt = ?
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

pub(super) fn select_id_by_remote_post(
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
         JOIN {REMOTE_TARGETS} t ON t.working_copy_rowid = w.rowid
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
