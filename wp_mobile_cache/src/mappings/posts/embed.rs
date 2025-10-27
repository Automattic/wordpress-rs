use crate::{
    DbSite, RowId, SqliteDbError,
    mappings::{
        ColumnIndex, RowExt,
        helpers::{get_id, get_optional_id},
    },
    term_relationships::DbTermRelationship,
};
use rusqlite::Row;
use wp_api::posts::{
    AnyPostWithEmbedContext, PostTitleWithEmbedContext, SparsePostExcerpt,
};

/// Column indexes for posts_embed_context table.
/// These must match the order of columns in the CREATE TABLE statement.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
enum PostEmbedContextColumn {
    Rowid = 0,
    SiteId = 1,
    Id = 2,
    Date = 3,
    Link = 4,
    Slug = 5,
    PostType = 6,
    TitleRendered = 7,
    Author = 8,
    ExcerptRaw = 9,
    ExcerptRendered = 10,
    ExcerptProtected = 11,
    FeaturedMedia = 12,
    LastFetchedAt = 13,
}

impl ColumnIndex for PostEmbedContextColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}

pub struct DbAnyPostWithEmbedContext {
    pub row_id: RowId,
    pub site: DbSite,
    pub post: AnyPostWithEmbedContext,
    pub last_fetched_at: String,
}

impl DbAnyPostWithEmbedContext {
    /// Construct a post entity from a database row.
    ///
    /// This is the only way to construct a `DbAnyPostWithEmbedContext`.
    ///
    /// Note: Embed context doesn't support term relationships (no categories/tags),
    /// but we accept the parameter for API consistency with other contexts.
    ///
    /// # Arguments
    /// * `row` - Database row containing post data
    /// * `term_relationships` - Ignored for embed context (no taxonomy support)
    pub fn from_row_with_terms(
        row: &Row,
        _term_relationships: Vec<DbTermRelationship>,
    ) -> Result<Self, SqliteDbError> {
        use PostEmbedContextColumn::*;

        let row_id: RowId = row.get_column(Rowid)?;
        let site = DbSite {
            row_id: row.get_column(PostEmbedContextColumn::SiteId)?,
        };

        let post = AnyPostWithEmbedContext {
            id: get_id(row, Id)?,
            date: row.get_column(Date)?,
            link: row.get_column(Link)?,
            slug: row.get_column(Slug)?,
            post_type: row.get_column(PostType)?,
            title: PostTitleWithEmbedContext {
                rendered: row.get_column(TitleRendered)?,
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
        };

        Ok(Self {
            row_id,
            site,
            post,
            last_fetched_at: row.get_column(LastFetchedAt)?,
        })
    }
}
