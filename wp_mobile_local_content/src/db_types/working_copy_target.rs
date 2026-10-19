//! Where a working copy is pushed.
//!
//! The target is a relationship, not something the user edits, so it lives in
//! its own table. Its rows depend on their working copy and are deleted with
//! it, never the other way around. Sites live in a table of their own, which
//! targets reference.

use std::sync::Arc;

use rusqlite::types::FromSqlError;
use wp_api::{date::WpGmtDateTime, parsed_url::ParsedUrl, posts::PostId, wp_com::WpComSiteId};
use wp_mobile_sqlite::ColumnIndex;

/// Identifies a site by something that survives the cache being cleared.
///
/// A site with a WordPress.com site ID is identified by it, whatever its URL,
/// so a working copy stays with its site when the site's URL changes. A site
/// without one is identified by its URL, until a site ID is given for that URL.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SiteIdentity {
    /// Stored in the serialized form `ParsedUrl` produces, so URLs that parse
    /// to the same `ParsedUrl` are stored the same way.
    pub url: Arc<ParsedUrl>,
    pub wordpress_com_site_id: Option<WpComSiteId>,
}

/// The remote post a working copy edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct RemotePostReference {
    pub post_id: PostId,
    /// The remote post's `modified_gmt` when the working copy was made from it.
    pub base_modified_gmt: WpGmtDateTime,
}

/// The site a working copy is pushed to, and the remote post it edits.
///
/// With a `remote_post`, pushing updates that post; without one, it creates a
/// new post.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct WorkingCopyTarget {
    pub site: SiteIdentity,
    pub remote_post: Option<RemotePostReference>,
}

impl SiteIdentity {
    /// The `wordpress_com_site_id` and `url` column values.
    pub(crate) fn to_columns(&self) -> (Option<i64>, &str) {
        (
            // SQLite stores integers as i64; WordPress.com site IDs fit.
            self.wordpress_com_site_id.map(|site_id| site_id.0 as i64),
            self.url.as_str(),
        )
    }

    pub(crate) fn from_columns(
        wordpress_com_site_id: Option<i64>,
        url: &str,
    ) -> Result<Self, FromSqlError> {
        Ok(Self {
            url: Arc::new(ParsedUrl::parse(url).map_err(|e| FromSqlError::Other(e.into()))?),
            wordpress_com_site_id: wordpress_com_site_id.map(|site_id| WpComSiteId(site_id as u64)),
        })
    }
}

/// Column indexes for the sites table.
///
/// `rowid` (index 0) is matched in the WHERE clause and never read.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum SiteColumn {
    WordPressComSiteId = 1,
    Url = 2,
}

impl ColumnIndex for SiteColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}

/// Column indexes for the working_copy_targets table.
///
/// `working_copy_rowid` (index 0) is matched in the WHERE clause and never read.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum WorkingCopyTargetColumn {
    SiteRowid = 1,
    RemotePostId = 2,
    BaseModifiedGmt = 3,
}

impl ColumnIndex for WorkingCopyTargetColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}
