use crate::{RowId, db_types::row_ext::ColumnIndex};
use wp_api::posts::AnyPostWithEmbedContext;

/// Column indexes for posts_embed_context table.
/// These must match the order of columns in the CREATE TABLE statement.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum PostEmbedContextColumn {
    Rowid = 0,
    DbSiteId = 1,
    Id = 2,
    Date = 3,
    Link = 4,
    Slug = 5,
    PostType = 6,
    Author = 7,
    ExcerptRaw = 8,
    ExcerptRendered = 9,
    ExcerptProtected = 10,
    FeaturedMedia = 11,
    LastFetchedAt = 12,
    TitleRendered = 13,
}

impl ColumnIndex for PostEmbedContextColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}

pub struct DbAnyPostWithEmbedContext {
    pub row_id: RowId,
    pub db_site_id: RowId,
    pub post: AnyPostWithEmbedContext,
    pub last_fetched_at: String,
}
