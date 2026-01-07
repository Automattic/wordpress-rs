use crate::RowId;
use wp_api::posts::AnyPostWithViewContext;

pub struct DbAnyPostWithViewContext {
    pub row_id: RowId,
    pub db_site_id: RowId,
    pub post: AnyPostWithViewContext,
    pub last_fetched_at: String,
}
