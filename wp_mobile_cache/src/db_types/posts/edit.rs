use crate::{DbSite, RowId, db_types::row_ext::ColumnIndex};
use wp_api::posts::AnyPostWithEditContext;

/// Column indexes for posts_edit_context table.
/// These must match the order of columns in the CREATE TABLE statement.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum PostEditContextColumn {
    Rowid = 0,
    SiteId = 1,
    Id = 2,
    Date = 3,
    DateGmt = 4,
    Link = 5,
    Modified = 6,
    ModifiedGmt = 7,
    Slug = 8,
    Status = 9,
    PostType = 10,
    Password = 11,
    Template = 12,
    PermalinkTemplate = 13,
    GeneratedSlug = 14,
    Author = 15,
    FeaturedMedia = 16,
    Sticky = 17,
    Parent = 18,
    MenuOrder = 19,
    CommentStatus = 20,
    PingStatus = 21,
    Format = 22,
    Meta = 23,
    GuidRaw = 24,
    GuidRendered = 25,
    TitleRaw = 26,
    TitleRendered = 27,
    ContentRaw = 28,
    ContentRendered = 29,
    ContentProtected = 30,
    ContentBlockVersion = 31,
    ExcerptRaw = 32,
    ExcerptRendered = 33,
    ExcerptProtected = 34,
    LastFetchedAt = 35,
}

impl ColumnIndex for PostEditContextColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}

pub struct DbAnyPostWithEditContext {
    pub row_id: RowId,
    pub site: DbSite,
    pub post: AnyPostWithEditContext,
    pub last_fetched_at: String,
}
