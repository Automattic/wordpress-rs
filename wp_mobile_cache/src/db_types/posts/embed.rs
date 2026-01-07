use crate::RowId;
use wp_api::posts::AnyPostWithEmbedContext;

pub struct DbAnyPostWithEmbedContext {
    pub row_id: RowId,
    pub db_site_id: RowId,
    pub post: AnyPostWithEmbedContext,
    pub last_fetched_at: String,
}
