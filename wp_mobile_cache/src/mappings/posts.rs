use crate::{
    DbSite, RowId, SqliteDbError,
    mappings::{
        ColumnIndex, RowExt, TryFromDbRow,
        helpers::{
            deserialize_json_value, get_id, get_optional_id, integer_to_bool, parse_datetime,
            parse_enum, parse_optional_enum,
        },
    },
    repository::DbEntity,
};
use rusqlite::Row;
use wp_api::posts::{
    AnyPostWithEditContext, PostContentWithEditContext, PostGuidWithEditContext,
    PostTitleWithEditContext, SparsePostExcerpt,
};

/// Column indexes for posts_edit_context table.
/// These must match the order of columns in the CREATE TABLE statement.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
enum PostEditContextColumn {
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

impl TryFromDbRow for DbAnyPostWithEditContext {
    fn try_from_row(row: &Row) -> Result<Self, SqliteDbError> {
        use PostEditContextColumn::*;

        let row_id: RowId = row.get_column(Rowid)?;
        let site = DbSite {
            row_id: row.get_column(PostEditContextColumn::SiteId)?,
        };

        let post = AnyPostWithEditContext {
            id: get_id(row, Id)?,
            date: row.get_column(Date)?,
            date_gmt: parse_datetime(row, DateGmt)?,
            guid: PostGuidWithEditContext {
                raw: row.get_column(GuidRaw)?,
                rendered: row.get_column(GuidRendered)?,
            },
            link: row.get_column(Link)?,
            modified: row.get_column(Modified)?,
            modified_gmt: parse_datetime(row, ModifiedGmt)?,
            slug: row.get_column(Slug)?,
            status: parse_enum(row, Status)?,
            post_type: row.get_column(PostType)?,
            password: row.get_column(Password)?,
            permalink_template: row.get_column(PermalinkTemplate)?,
            generated_slug: row.get_column(GeneratedSlug)?,
            title: PostTitleWithEditContext {
                raw: row.get_column(TitleRaw)?,
                rendered: row.get_column(TitleRendered)?,
            },
            content: PostContentWithEditContext {
                raw: row.get_column(ContentRaw)?,
                rendered: row.get_column(ContentRendered)?,
                protected: row.get_column(ContentProtected)?,
                block_version: row.get_column(ContentBlockVersion)?,
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
            categories: None,
            tags: None,
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

impl DbEntity for AnyPostWithEditContext {
    const TABLE_NAME: &'static str = "posts_edit_context";
}

#[cfg(test)]
mod tests {
    use crate::{
        repository::posts::PostRepository,
        test_fixtures::posts::{create_full_post, create_minimal_post},
        test_helpers::{TestContext, test_ctx},
    };
    use rstest::*;
    use wp_api::posts::{PostId, PostStatus};

    /// Helper to validate that last_fetched_at is a recent, valid ISO 8601 timestamp
    fn assert_recent_timestamp(timestamp: &str) {
        // Parse the timestamp
        assert!(
            timestamp.ends_with('Z'),
            "Timestamp should be UTC (end with Z): {}",
            timestamp
        );
        assert!(
            timestamp.contains('T'),
            "Timestamp should be ISO 8601 format: {}",
            timestamp
        );
        // Basic format check: YYYY-MM-DDTHH:MM:SS.fffZ
        assert!(
            timestamp.len() >= 20,
            "Timestamp should be at least 20 chars: {}",
            timestamp
        );
    }

    #[rstest]
    fn test_round_trip_with_minimal_fields(mut test_ctx: TestContext) {
        let repo = PostRepository;
        let original_post = create_minimal_post();

        // Insert into database using repository
        let rowid = repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &original_post)
            .expect("Failed to insert post");

        // Read back from database using PostRepository's select_by_rowid
        let retrieved = repo
            .select_by_rowid(&test_ctx.conn, &test_ctx.site, rowid)
            .expect("Failed to read post");

        // Verify round-trip
        assert_eq!(retrieved.row_id, rowid);
        assert_eq!(retrieved.site, test_ctx.site);
        assert_recent_timestamp(&retrieved.last_fetched_at);
        assert_eq!(retrieved.post, original_post);
    }

    #[rstest]
    fn test_round_trip_with_all_fields(mut test_ctx: TestContext) {
        let repo = PostRepository;
        let original_post = create_full_post();

        // Insert into database using repository
        let rowid = repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &original_post)
            .expect("Failed to insert post");

        // Read back from database using repository
        let retrieved = repo
            .select_by_rowid(&test_ctx.conn, &test_ctx.site, rowid)
            .expect("Failed to read post");

        // Verify round-trip for all fields
        assert_eq!(retrieved.row_id, rowid);
        assert_eq!(retrieved.site, test_ctx.site);
        assert_recent_timestamp(&retrieved.last_fetched_at);
        assert_eq!(retrieved.post, original_post);
    }

    #[rstest]
    fn test_round_trip_with_optional_fields_none(mut test_ctx: TestContext) {
        let repo = PostRepository;
        let mut post = create_minimal_post();
        post.id = PostId(99);

        // Explicitly set all optional fields to None
        post.permalink_template = None;
        post.generated_slug = None;
        post.author = None;
        post.excerpt = None;
        post.featured_media = None;
        post.comment_status = None;
        post.ping_status = None;
        post.format = None;
        post.meta = None;
        post.sticky = None;
        post.categories = None;
        post.tags = None;
        post.parent = None;
        post.menu_order = None;

        // Insert and retrieve using repository
        let rowid = repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .expect("Failed to insert post");
        let retrieved = repo
            .select_by_rowid(&test_ctx.conn, &test_ctx.site, rowid)
            .expect("Failed to read post");

        // All optional fields should still be None
        assert_eq!(retrieved.post, post);
    }

    #[rstest]
    fn test_round_trip_with_different_enum_variants(mut test_ctx: TestContext) {
        let repo = PostRepository;

        // Test with different status variants
        let statuses = [
            PostStatus::Publish,
            PostStatus::Draft,
            PostStatus::Pending,
            PostStatus::Private,
            PostStatus::Future,
            PostStatus::Custom("custom-status".to_string()),
        ];

        for (i, status) in statuses.iter().enumerate() {
            let mut post = create_minimal_post();
            post.id = PostId((100 + i) as i64);
            post.status = status.clone();

            let rowid = repo
                .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
                .unwrap();
            let retrieved = repo
                .select_by_rowid(&test_ctx.conn, &test_ctx.site, rowid)
                .unwrap();

            assert_eq!(retrieved.post.status, *status);
        }
    }

    #[rstest]
    fn test_round_trip_with_empty_json_arrays(mut test_ctx: TestContext) {
        let repo = PostRepository;
        let mut post = create_minimal_post();
        post.id = PostId(200);
        post.categories = Some(vec![]);
        post.tags = Some(vec![]);

        let rowid = repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();
        let retrieved = repo
            .select_by_rowid(&test_ctx.conn, &test_ctx.site, rowid)
            .unwrap();

        assert_eq!(retrieved.post.categories, None);
        assert_eq!(retrieved.post.tags, None);
    }

    #[rstest]
    fn test_round_trip_with_sticky_boolean_variants(mut test_ctx: TestContext) {
        let repo = PostRepository;

        // Test sticky = Some(true)
        let mut post = create_minimal_post();
        post.id = PostId(300);
        post.sticky = Some(true);

        let rowid = repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();
        let retrieved = repo
            .select_by_rowid(&test_ctx.conn, &test_ctx.site, rowid)
            .unwrap();
        assert_eq!(retrieved.post.sticky, Some(true));

        // Test sticky = Some(false)
        let mut post = create_minimal_post();
        post.id = PostId(301);
        post.sticky = Some(false);

        let rowid = repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();
        let retrieved = repo
            .select_by_rowid(&test_ctx.conn, &test_ctx.site, rowid)
            .unwrap();
        assert_eq!(retrieved.post.sticky, Some(false));

        // Test sticky = None
        let mut post = create_minimal_post();
        post.id = PostId(302);
        post.sticky = None;

        let rowid = repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();
        let retrieved = repo
            .select_by_rowid(&test_ctx.conn, &test_ctx.site, rowid)
            .unwrap();
        assert_eq!(retrieved.post.sticky, None);
    }
}
