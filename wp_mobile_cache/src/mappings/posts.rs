use crate::{
    DbSite, RowId, SqliteDbError,
    mappings::{
        ColumnIndex, InsertIntoDb, RowExt, TryFromDbRow,
        helpers::{
            bool_to_integer, deserialize_json_id_array, deserialize_json_value, get_id,
            get_optional_id, integer_to_bool, parse_datetime, parse_enum, parse_optional_enum,
            serialize_json_id_array, serialize_value_to_json,
        },
    },
    repository::{DbEntity, QueryExecutor},
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
    Categories = 24,
    Tags = 25,
    GuidRaw = 26,
    GuidRendered = 27,
    TitleRaw = 28,
    TitleRendered = 29,
    ContentRaw = 30,
    ContentRendered = 31,
    ContentProtected = 32,
    ContentBlockVersion = 33,
    ExcerptRaw = 34,
    ExcerptRendered = 35,
    ExcerptProtected = 36,
    LastFetchedAt = 37,
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
            categories: deserialize_json_id_array(row.get_column(Categories)?)?,
            tags: deserialize_json_id_array(row.get_column(Tags)?)?,
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

impl InsertIntoDb for AnyPostWithEditContext {
    fn insert_into_db(
        &self,
        executor: &impl crate::repository::QueryExecutor,
        site: &DbSite,
    ) -> Result<RowId, SqliteDbError> {
        executor.execute(
            r#"
            INSERT INTO posts_edit_context (
                db_site_id, id, date, date_gmt, link, modified, modified_gmt, slug, status, post_type,
                password, template, permalink_template, generated_slug, author, featured_media,
                sticky, parent, menu_order, comment_status, ping_status, format, meta,
                categories, tags, guid_raw, guid_rendered, title_raw, title_rendered,
                content_raw, content_rendered, content_protected, content_block_version,
                excerpt_raw, excerpt_rendered, excerpt_protected
            ) VALUES (
                :db_site_id, :id, :date, :date_gmt, :link, :modified, :modified_gmt, :slug, :status, :post_type,
                :password, :template, :permalink_template, :generated_slug, :author, :featured_media,
                :sticky, :parent, :menu_order, :comment_status, :ping_status, :format, :meta,
                :categories, :tags, :guid_raw, :guid_rendered, :title_raw, :title_rendered,
                :content_raw, :content_rendered, :content_protected, :content_block_version,
                :excerpt_raw, :excerpt_rendered, :excerpt_protected
            )
            "#,
            rusqlite::named_params! {
                ":db_site_id": site.row_id,
                ":id": self.id.0,
                ":date": self.date,
                ":date_gmt": self.date_gmt.to_string(),
                ":link": self.link,
                ":modified": self.modified,
                ":modified_gmt": self.modified_gmt.to_string(),
                ":slug": self.slug,
                ":status": self.status.to_string(),
                ":post_type": self.post_type,
                ":password": self.password,
                ":template": self.template,
                ":permalink_template": self.permalink_template,
                ":generated_slug": self.generated_slug,
                ":author": self.author.map(|u| u.0),
                ":featured_media": self.featured_media.map(|m| m.0),
                ":sticky": bool_to_integer(self.sticky),
                ":parent": self.parent.map(|p| p.0),
                ":menu_order": self.menu_order,
                ":comment_status": self.comment_status.as_ref().map(|s| s.to_string()),
                ":ping_status": self.ping_status.as_ref().map(|s| s.to_string()),
                ":format": self.format.as_ref().map(|f| f.to_string()),
                ":meta": serialize_value_to_json(&self.meta)?,
                ":categories": serialize_json_id_array(&self.categories, |t| t.0)?,
                ":tags": serialize_json_id_array(&self.tags, |t| t.0)?,
                ":guid_raw": self.guid.raw,
                ":guid_rendered": self.guid.rendered,
                ":title_raw": self.title.raw,
                ":title_rendered": self.title.rendered,
                ":content_raw": self.content.raw,
                ":content_rendered": self.content.rendered,
                ":content_protected": self.content.protected,
                ":content_block_version": self.content.block_version,
                ":excerpt_raw": self.excerpt.as_ref().and_then(|e| e.raw.clone()),
                ":excerpt_rendered": self.excerpt.as_ref().and_then(|e| e.rendered.clone()),
                ":excerpt_protected": self.excerpt.as_ref().and_then(|e| e.protected),
            },
        )?;

        Ok(QueryExecutor::last_insert_rowid(executor))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        DbSite,
        repository::{Repository, posts::PostRepository},
        test_fixtures::posts::{create_full_post, create_minimal_post},
        test_helpers::{test_db, test_site},
    };
    use rstest::*;
    use rusqlite::Connection;
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
    fn test_round_trip_with_minimal_fields(test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;
        let original_post = create_minimal_post();

        // Insert into database using repository
        let rowid = repo
            .insert(&test_db, &original_post, &test_site)
            .expect("Failed to insert post");

        // Read back from database using PostRepository's select_by_rowid
        let retrieved = repo
            .select_by_rowid(&test_db, &test_site, rowid)
            .expect("Failed to read post");

        // Verify round-trip
        assert_eq!(retrieved.row_id, rowid);
        assert_eq!(retrieved.site, test_site);
        assert_recent_timestamp(&retrieved.last_fetched_at);
        assert_eq!(retrieved.post, original_post);
    }

    #[rstest]
    fn test_round_trip_with_all_fields(test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;
        let original_post = create_full_post();

        // Insert into database using repository
        let rowid = repo
            .insert(&test_db, &original_post, &test_site)
            .expect("Failed to insert post");

        // Read back from database using repository
        let retrieved = repo
            .select_by_rowid(&test_db, &test_site, rowid)
            .expect("Failed to read post");

        // Verify round-trip for all fields
        assert_eq!(retrieved.row_id, rowid);
        assert_eq!(retrieved.site, test_site);
        assert_recent_timestamp(&retrieved.last_fetched_at);
        assert_eq!(retrieved.post, original_post);
    }

    #[rstest]
    fn test_round_trip_with_optional_fields_none(test_db: Connection, test_site: DbSite) {
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
            .insert(&test_db, &post, &test_site)
            .expect("Failed to insert post");
        let retrieved = repo
            .select_by_rowid(&test_db, &test_site, rowid)
            .expect("Failed to read post");

        // All optional fields should still be None
        assert_eq!(retrieved.post, post);
    }

    #[rstest]
    fn test_round_trip_with_different_enum_variants(test_db: Connection, test_site: DbSite) {
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

            let rowid = repo.insert(&test_db, &post, &test_site).unwrap();
            let retrieved = repo.select_by_rowid(&test_db, &test_site, rowid).unwrap();

            assert_eq!(retrieved.post.status, *status);
        }
    }

    #[rstest]
    fn test_round_trip_with_empty_json_arrays(test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;
        let mut post = create_minimal_post();
        post.id = PostId(200);
        post.categories = Some(vec![]);
        post.tags = Some(vec![]);

        let rowid = repo.insert(&test_db, &post, &test_site).unwrap();
        let retrieved = repo.select_by_rowid(&test_db, &test_site, rowid).unwrap();

        assert_eq!(retrieved.post.categories, Some(vec![]));
        assert_eq!(retrieved.post.tags, Some(vec![]));
    }

    #[rstest]
    fn test_round_trip_with_sticky_boolean_variants(test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;

        // Test sticky = Some(true)
        let mut post = create_minimal_post();
        post.id = PostId(300);
        post.sticky = Some(true);

        let rowid = repo.insert(&test_db, &post, &test_site).unwrap();
        let retrieved = repo.select_by_rowid(&test_db, &test_site, rowid).unwrap();
        assert_eq!(retrieved.post.sticky, Some(true));

        // Test sticky = Some(false)
        let mut post = create_minimal_post();
        post.id = PostId(301);
        post.sticky = Some(false);

        let rowid = repo.insert(&test_db, &post, &test_site).unwrap();
        let retrieved = repo.select_by_rowid(&test_db, &test_site, rowid).unwrap();
        assert_eq!(retrieved.post.sticky, Some(false));

        // Test sticky = None
        let mut post = create_minimal_post();
        post.id = PostId(302);
        post.sticky = None;

        let rowid = repo.insert(&test_db, &post, &test_site).unwrap();
        let retrieved = repo.select_by_rowid(&test_db, &test_site, rowid).unwrap();
        assert_eq!(retrieved.post.sticky, None);
    }
}
