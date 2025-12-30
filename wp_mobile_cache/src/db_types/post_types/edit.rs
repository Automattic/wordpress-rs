use crate::RowId;
use wp_api::post_types::PostTypeDetailsWithEditContext;

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
