use crate::RowId;
use wp_api::posts::AnyPostWithEditContext;

pub struct DbAnyPostWithEditContext {
    pub row_id: RowId,
    pub db_site_id: RowId,
    pub post: AnyPostWithEditContext,
    pub last_fetched_at: String,
}
