//! Reading and writing `sites` rows, and finding the row a `SiteIdentity`
//! matches.

use rusqlite::OptionalExtension;
use rusqlite::types::Type;
use wp_mobile_sqlite::RowExt;

use super::{SITES, conversion_error};
use crate::{
    LocalContentError,
    db_types::working_copy_remote_target::{SiteColumn, SiteIdentity},
    repository::{InTransaction, QueryExecutor},
};

/// Returns the `sites` row `site` identifies, if there is one: the row with
/// its WordPress.com site ID, or else the row with its URL and no site ID.
pub(super) fn select_site_rowid(
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
pub(super) fn upsert_site(
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

pub(super) fn select_site(
    executor: &impl QueryExecutor,
    site_rowid: i64,
) -> Result<SiteIdentity, LocalContentError> {
    let mut stmt = executor.prepare(&format!("SELECT * FROM {SITES} WHERE rowid = ?"))?;
    let site = stmt.query_row([site_rowid], |row| {
        use SiteColumn::*;

        let url: String = row.get_column(Url)?;
        SiteIdentity::from_columns(row.get_column(WordPressComSiteId)?, &url)
            .map_err(|e| conversion_error(Url, Type::Text, e))
    })?;
    Ok(site)
}
