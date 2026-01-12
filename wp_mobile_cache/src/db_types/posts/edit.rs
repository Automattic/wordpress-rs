use crate::{RowId, db_types::row_ext::ColumnIndex};
use wp_api::posts::AnyPostWithEditContext;

/// Column indexes for posts_edit_context table.
/// These must match the order of columns in the CREATE TABLE statement.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum PostEditContextColumn {
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
    PermalinkTemplate = 12,
    GeneratedSlug = 13,
    Author = 14,
    FeaturedMedia = 15,
    Sticky = 16,
    Parent = 17,
    MenuOrder = 18,
    CommentStatus = 19,
    PingStatus = 20,
    Format = 21,
    Meta = 22,
    GuidRaw = 23,
    GuidRendered = 24,
    TitleRaw = 25,
    ContentRaw = 26,
    ContentRendered = 27,
    ContentProtected = 28,
    ContentBlockVersion = 29,
    ExcerptRaw = 30,
    ExcerptRendered = 31,
    ExcerptProtected = 32,
    LastFetchedAt = 33,
    TitleRendered = 34,
    Password = 35,
}

impl ColumnIndex for PostEditContextColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}

pub struct DbAnyPostWithEditContext {
    pub row_id: RowId,
    pub db_site_id: RowId,
    pub post: AnyPostWithEditContext,
    pub last_fetched_at: String,
}
