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
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SiteIdentity {
    WordPressCom {
        site_id: WpComSiteId,
    },
    /// Stored in the serialized form `ParsedUrl` produces, so URLs that parse
    /// to the same `ParsedUrl` are stored the same way.
    SelfHosted {
        url: Arc<ParsedUrl>,
    },
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
    /// The `site_kind`, `wordpress_com_site_id` and `self_hosted_url` column values.
    pub(crate) fn to_columns(&self) -> (&'static str, Option<i64>, Option<&str>) {
        match self {
            // SQLite stores integers as i64; WordPress.com site IDs fit.
            Self::WordPressCom { site_id } => ("wordpress_com", Some(site_id.0 as i64), None),
            Self::SelfHosted { url } => ("self_hosted", None, Some(url.as_str())),
        }
    }

    pub(crate) fn from_columns(
        site_kind: &str,
        wordpress_com_site_id: Option<i64>,
        self_hosted_url: Option<String>,
    ) -> Result<Self, FromSqlError> {
        match (site_kind, wordpress_com_site_id, self_hosted_url) {
            ("wordpress_com", Some(site_id), None) => Ok(Self::WordPressCom {
                site_id: WpComSiteId(site_id as u64),
            }),
            ("self_hosted", None, Some(url)) => Ok(Self::SelfHosted {
                url: Arc::new(ParsedUrl::parse(&url).map_err(|e| FromSqlError::Other(e.into()))?),
            }),
            (site_kind, _, _) => Err(FromSqlError::Other(
                format!("Inconsistent site columns for site_kind={site_kind}").into(),
            )),
        }
    }
}

/// Column indexes for the sites table.
///
/// `rowid` (index 0) is matched in the WHERE clause and never read.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum SiteColumn {
    SiteKind = 1,
    WordPressComSiteId = 2,
    SelfHostedUrl = 3,
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
