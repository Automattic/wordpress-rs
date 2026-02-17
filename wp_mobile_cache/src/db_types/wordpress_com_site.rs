use crate::{RowId, db_types::row_ext::ColumnIndex};
use wp_api::wp_com::WpComSiteId;

/// Column indexes for wordpress_com_sites table.
/// These must match the order of columns in the CREATE TABLE statement.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum DbWordPressComSiteColumn {
    Rowid = 0,
    SiteId = 1,
}

impl ColumnIndex for DbWordPressComSiteColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}

/// Represents a WordPress.com site (domain model).
///
/// This type contains the site data without database metadata.
/// Use this when creating or updating sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordPressComSite {
    pub site_id: WpComSiteId,
}

/// Represents a WordPress.com site in the database.
///
/// This type includes the database rowid along with the site data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbWordPressComSite {
    pub row_id: RowId,
    pub site_id: WpComSiteId,
}
