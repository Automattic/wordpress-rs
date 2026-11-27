//! Remote targets, and the site identities and remote post references they hold.

use std::sync::Arc;

use rusqlite::types::FromSqlError;
use wp_api::{date::WpGmtDateTime, parsed_url::ParsedUrl, posts::PostId, wp_com::WpComSiteId};
use wp_mobile_sqlite::ColumnIndex;

/// The site a working copy belongs to, identified by values that stay valid
/// when the cache is cleared.
///
/// A site with a WordPress.com site ID is matched by that ID, so its working
/// copies follow it to a new URL. Set `wordpress_com_site_id` whenever the site
/// has one, for reads as well as writes. Once a working copy is created with a
/// site ID, the site's URL without the ID no longer finds it.
///
/// A site without a WordPress.com site ID is matched by its URL.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SiteIdentity {
    /// Matched after `ParsedUrl` normalizes it, so `HTTPS://EXAMPLE.COM/` and
    /// `https://example.com` are the same site.
    pub url: Arc<ParsedUrl>,
    pub wordpress_com_site_id: Option<WpComSiteId>,
}

/// A post on the server that a working copy updates when it's pushed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct RemotePostReference {
    pub post_id: PostId,
    /// The working copy's base version: the `modified_gmt` of the server
    /// version its fields start from.
    ///
    /// It's the post's `modified_gmt` when the working copy was created from
    /// it, and after a recorded push, the `modified_gmt` the server returned. If
    /// the post's current `modified_gmt` is different, the post has changed on
    /// the server since.
    pub base_modified_gmt: WpGmtDateTime,
}

/// Where a working copy goes when it's pushed: its site, and the post on that
/// site the push updates.
///
/// Without a `remote_post`, the working copy is a new post that isn't on the
/// server yet. Recording its first push sets `remote_post` to the post that
/// push created.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct WorkingCopyRemoteTarget {
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

/// Column indexes for the working_copy_remote_targets table.
///
/// `working_copy_rowid` (index 0) is matched in the WHERE clause and never read.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum WorkingCopyRemoteTargetColumn {
    SiteRowid = 1,
    RemotePostId = 2,
    BaseModifiedGmt = 3,
}

impl ColumnIndex for WorkingCopyRemoteTargetColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}
