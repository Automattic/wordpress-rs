use crate::{
    DbSite, RowId, SqliteDbError,
    mappings::{
        ColumnIndex, RowExt,
        helpers::{
            deserialize_json_value, get_id, get_optional_id, integer_to_bool, parse_datetime,
            parse_enum, parse_optional_enum,
        },
    },
    term_relationships::DbTermRelationship,
};
use rusqlite::Row;
use wp_api::posts::{
    AnyPostWithViewContext, PostContentWithViewContext, PostGuidWithViewContext,
    PostTitleWithViewContext, SparsePostExcerpt,
};

/// Column indexes for posts_view_context table.
/// These must match the order of columns in the CREATE TABLE statement.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
enum PostViewContextColumn {
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
    pub site: DbSite,
    pub post: AnyPostWithViewContext,
    pub last_fetched_at: String,
}

impl DbAnyPostWithViewContext {
    /// Construct a post entity from a database row with its associated term relationships.
    ///
    /// This is the only way to construct a `DbAnyPostWithViewContext`, ensuring that
    /// terms are always properly loaded from the term_relationships table.
    ///
    /// Domain-specific logic for extracting categories and tags from the generic
    /// term relationships is handled here in the mapping layer.
    ///
    /// # Arguments
    /// * `row` - Database row containing post data
    /// * `term_relationships` - Term relationships loaded from term_relationships table
    pub fn from_row_with_terms(
        row: &Row,
        term_relationships: Vec<DbTermRelationship>,
    ) -> Result<Self, SqliteDbError> {
        use PostViewContextColumn::*;

        let row_id: RowId = row.get_column(Rowid)?;
        let site = DbSite {
            row_id: row.get_column(PostViewContextColumn::SiteId)?,
        };

        // Extract categories and tags from term relationships
        let (categories, tags) = super::extract_categories_and_tags(term_relationships);

        let post = AnyPostWithViewContext {
            id: get_id(row, Id)?,
            date: row.get_column(Date)?,
            date_gmt: parse_datetime(row, DateGmt)?,
            guid: PostGuidWithViewContext {
                rendered: row.get_column(GuidRendered)?,
            },
            link: row.get_column(Link)?,
            modified: row.get_column(Modified)?,
            modified_gmt: parse_datetime(row, ModifiedGmt)?,
            slug: row.get_column(Slug)?,
            status: parse_enum(row, Status)?,
            post_type: row.get_column(PostType)?,
            title: PostTitleWithViewContext {
                rendered: row.get_column(TitleRendered)?,
            },
            content: PostContentWithViewContext {
                rendered: row.get_column(ContentRendered)?,
                protected: row.get_column(ContentProtected)?,
            },
            author: get_optional_id(row, Author)?,
            excerpt: {
                // Presence of excerpt is determined by excerpt_rendered being Some
                let excerpt_rendered: Option<String> = row.get_column(ExcerptRendered)?;
                if excerpt_rendered.is_some() {
                    Some(SparsePostExcerpt {
                        raw: row.get_column(ExcerptRaw)?,
                        rendered: excerpt_rendered,
                        protected: row.get_column(ExcerptProtected)?,
                    })
                } else {
                    None
                }
            },
            featured_media: get_optional_id(row, FeaturedMedia)?,
            comment_status: parse_optional_enum(row, CommentStatus)?,
            ping_status: parse_optional_enum(row, PingStatus)?,
            format: parse_optional_enum(row, Format)?,
            meta: deserialize_json_value(row.get_column(Meta)?)?,
            sticky: integer_to_bool(row.get_column(Sticky)?),
            template: row.get_column(Template)?,
            categories: if categories.is_empty() {
                None
            } else {
                Some(categories)
            },
            tags: if tags.is_empty() { None } else { Some(tags) },
            parent: get_optional_id(row, Parent)?,
            menu_order: row.get_column(MenuOrder)?,
        };

        Ok(Self {
            row_id,
            site,
            post,
            last_fetched_at: row.get_column(LastFetchedAt)?,
        })
    }
}
