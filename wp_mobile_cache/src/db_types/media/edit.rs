use crate::{RowId, db_types::row_ext::ColumnIndex};
use wp_api::media::MediaWithEditContext;

/// Column indexes for media_edit_context table.
/// These must match the order of columns in the CREATE TABLE statement.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum MediaEditContextColumn {
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
    Password = 11,
    PermalinkTemplate = 12,
    GeneratedSlug = 13,
    Author = 14,
    CommentStatus = 15,
    PingStatus = 16,
    Template = 17,
    AltText = 18,
    MediaType = 19,
    MimeType = 20,
    SourceUrl = 21,
    PostId = 22,
    MissingImageSizes = 23,
    GuidRaw = 24,
    GuidRendered = 25,
    TitleRaw = 26,
    TitleRendered = 27,
    CaptionRaw = 28,
    CaptionRendered = 29,
    DescriptionRaw = 30,
    DescriptionRendered = 31,
    MediaDetails = 32,
    LastFetchedAt = 33,
    AdditionalFields = 34,
}

impl ColumnIndex for MediaEditContextColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}

pub struct DbMediaWithEditContext {
    pub row_id: RowId,
    pub db_site_id: RowId,
    pub media: MediaWithEditContext,
    pub last_fetched_at: String,
}
