use crate::{RowId, db_types::row_ext::ColumnIndex};
use wp_api::posts::AnyPostWithViewContext;

/// Column indexes for posts_view_context table.
/// These must match the order of columns in the CREATE TABLE statement.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum PostViewContextColumn {
    Rowid = 0,
    DbSiteId = 1,
    Id = 2,
    Date = 3,
    DateGmt = 4,
    Link = 5,
    Modified = 6,
    ModifiedGmt = 7,
    Slug = 8,
    Status = 9,
    PostType = 10,
    Template = 11,
    Author = 12,
    FeaturedMedia = 13,
    Sticky = 14,
    Parent = 15,
    MenuOrder = 16,
    CommentStatus = 17,
    PingStatus = 18,
    Format = 19,
    Meta = 20,
    GuidRendered = 21,
    TitleRendered = 22,
    ContentRendered = 23,
    ContentProtected = 24,
    ExcerptRaw = 25,
    ExcerptRendered = 26,
    ExcerptProtected = 27,
    LastFetchedAt = 28,
}

impl ColumnIndex for PostViewContextColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}

pub struct DbAnyPostWithViewContext {
    pub row_id: RowId,
    pub db_site_id: RowId,
    pub post: AnyPostWithViewContext,
    pub last_fetched_at: String,
}
