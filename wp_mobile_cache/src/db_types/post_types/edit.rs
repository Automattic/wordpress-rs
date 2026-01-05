use crate::{RowId, db_types::row_ext::ColumnIndex};
use wp_api::post_types::PostTypeDetailsWithEditContext;

/// Column indexes for post_types_edit_context table.
/// These must match the order of columns in the CREATE TABLE statement.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum PostTypeEditContextColumn {
    Rowid = 0,
    DbSiteId = 1,
    Slug = 2,
    Data = 3,
    LastFetchedAt = 4,
}

impl ColumnIndex for PostTypeEditContextColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}

/// Database representation of a post type with edit context.
///
/// This wraps the API type along with database metadata (rowid, site_id, last_fetched_at).
pub struct DbPostTypeDetailsWithEditContext {
    pub row_id: RowId,
    pub db_site_id: RowId,
    pub slug: String,
    pub post_type: PostTypeDetailsWithEditContext,
    pub last_fetched_at: String,
}
