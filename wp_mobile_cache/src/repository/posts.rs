use crate::{
    DbSite, RowId, SqliteDbError,
    context::PostContext,
    db_types::posts::{
        DbAnyPostWithEditContext, DbAnyPostWithEmbedContext, DbAnyPostWithViewContext,
        PostEditContextColumn, PostEmbedContextColumn, PostViewContextColumn,
    },
    mappings::{
        RowExt,
        helpers::{
            bool_to_integer, deserialize_json_value, get_id, get_optional_id, integer_to_bool,
            parse_datetime, parse_enum, parse_optional_enum, serialize_value_to_json,
        },
    },
    repository::{
        QueryExecutor, TransactionManager, term_relationships::TermRelationshipRepository,
    },
    term_relationships::DbTermRelationship,
};
use rusqlite::{OptionalExtension, Row};
use std::marker::PhantomData;
use wp_api::{
    posts::{
        AnyPostWithEditContext, AnyPostWithEmbedContext, AnyPostWithViewContext,
        PostContentWithEditContext, PostContentWithViewContext,
        PostGuidWithEditContext, PostGuidWithViewContext,
        PostId, PostTitleWithEditContext, PostTitleWithEmbedContext, PostTitleWithViewContext,
        SparsePostExcerpt,
    },
    taxonomies::TaxonomyType,
    terms::TermId,
};

// Re-export for public API
pub use crate::db_types::posts::{
    DbAnyPostWithEditContext as DbPostEdit, DbAnyPostWithEmbedContext as DbPostEmbed,
    DbAnyPostWithViewContext as DbPostView,
};

/// Extract categories and tags from term relationships.
fn extract_categories_and_tags(
    term_relationships: Vec<DbTermRelationship>,
) -> (Vec<TermId>, Vec<TermId>) {
    term_relationships.into_iter().fold(
        (Vec::new(), Vec::new()),
        |(mut cats, mut tags), relationship| {
            match relationship.taxonomy_type {
                TaxonomyType::Category => cats.push(relationship.term_id),
                TaxonomyType::PostTag => tags.push(relationship.term_id),
                _ => {} // Ignore other taxonomy types for posts
            }
            (cats, tags)
        },
    )
}

/// Repository for managing posts in the database.
///
/// Generic over PostContext trait to support edit, view, and embed contexts.
/// Each context provides appropriate type associations.
///
/// # Type Parameters
/// * `C` - The context type (EditContext, ViewContext, or EmbedContext)
pub struct PostRepository<C: PostContext> {
    _phantom: PhantomData<C>,
}

impl<C: PostContext> Default for PostRepository<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: PostContext> PostRepository<C> {
    const TABLE_NAME_PREFIX: &'static str = "posts";

    /// Create a new repository instance.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }

    /// Get the full table name for this context.
    fn table_name() -> String {
        C::table_name(Self::TABLE_NAME_PREFIX)
    }

    /// Select a post by its SQLite rowid for a given site (returns wrapper with rowid).
    ///
    /// Returns `Ok(None)` if no post with the given rowid exists for this site.
    /// Automatically populates categories and tags from term_relationships table.
    pub fn select_by_rowid(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        rowid: RowId,
    ) -> Result<Option<C::DbPost>, SqliteDbError> {
        // First get the post.id (WordPress ID) from the rowid
        let sql = format!(
            "SELECT id FROM {} WHERE db_site_id = ? AND rowid = ?",
            Self::table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        let Some(post_id) = stmt
            .query_row([site.row_id, rowid], |row| row.get(0))
            .optional()
            .map_err(SqliteDbError::from)? else {
            return Ok(None);
        };

        // Query and construct post with lazy term relationship loading
        let sql = format!(
            "SELECT * FROM {} WHERE db_site_id = ? AND rowid = ?",
            Self::table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        stmt.query_row([site.row_id, rowid], |row| {
            C::from_row_with_terms(row, || {
                let term_repo = TermRelationshipRepository;
                let terms_map = term_repo.get_terms_for_objects(executor, site, &[post_id])?;
                Ok(terms_map.get(&post_id).cloned().unwrap_or_default())
            })
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })
        .optional()
        .map_err(SqliteDbError::from)
    }

    /// Select all posts for a given site (returns wrappers with rowids).
    ///
    /// Returns an empty vector if no posts exist for the site.
    /// Automatically populates categories and tags from term_relationships table.
    pub fn select_all(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
    ) -> Result<Vec<C::DbPost>, SqliteDbError> {
        // First pass: extract post IDs (WordPress IDs, not SQLite rowids)
        let sql = format!("SELECT id FROM {} WHERE db_site_id = ?", Self::table_name());
        let mut stmt = executor.prepare(&sql)?;
        let post_ids: Vec<i64> = stmt
            .query_map([site.row_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(SqliteDbError::from)?;

        if post_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Batch load term relationships for all posts using WordPress post IDs
        // This is done upfront for efficiency, but each context decides whether to use them
        let term_repo = TermRelationshipRepository;
        let terms_map = term_repo.get_terms_for_objects(executor, site, &post_ids)?;

        // Second pass: construct posts with lazy term relationship access
        let sql = format!("SELECT * FROM {} WHERE db_site_id = ?", Self::table_name());
        let mut stmt = executor.prepare(&sql)?;
        let posts = stmt
            .query_map([site.row_id], |row| {
                let post_id: i64 = row.get("id")?;
                C::from_row_with_terms(row, || {
                    Ok(terms_map.get(&post_id).cloned().unwrap_or_default())
                })
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(SqliteDbError::from)?;

        Ok(posts)
    }

    /// Select a post by its WordPress post ID for a given site (returns wrapper with rowid).
    ///
    /// This is different from `select_by_rowid` which uses the SQLite rowid.
    /// The post_id is the WordPress post ID from the REST API.
    ///
    /// Returns `Ok(None)` if no post with the given WordPress post ID exists for this site.
    /// Automatically populates categories and tags from term_relationships table.
    pub fn select_by_post_id(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        post_id: PostId,
    ) -> Result<Option<C::DbPost>, SqliteDbError> {
        // Query and construct post with lazy term relationship loading
        let sql = format!(
            "SELECT * FROM {} WHERE db_site_id = ? AND id = ?",
            Self::table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        stmt.query_row(rusqlite::params![site.row_id, post_id.0], |row| {
            C::from_row_with_terms(row, || {
                let term_repo = TermRelationshipRepository;
                let terms_map = term_repo.get_terms_for_objects(executor, site, &[post_id.0])?;
                Ok(terms_map.get(&post_id.0).cloned().unwrap_or_default())
            })
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })
        .optional()
        .map_err(SqliteDbError::from)
    }

    /// Delete a post by its WordPress post ID for a given site.
    ///
    /// Returns the number of rows deleted (0 or 1).
    /// Automatically deletes associated term relationships.
    pub fn delete_by_post_id(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        post_id: PostId,
    ) -> Result<usize, SqliteDbError> {
        // First, try to get the rowid (if post doesn't exist, return 0)
        let _db_post = match self.select_by_post_id(executor, site, post_id)? {
            Some(post) => post,
            None => return Ok(0), // Post doesn't exist
        };

        // Delete term relationships using WordPress post ID
        let term_repo = TermRelationshipRepository;
        term_repo.delete_all_terms_for_object(executor, site, post_id.0)?;

        // Delete the post
        let sql = format!(
            "DELETE FROM {} WHERE db_site_id = ? AND id = ?",
            Self::table_name()
        );
        executor.execute(&sql, rusqlite::params![site.row_id, post_id.0])
    }

    /// Get the total count of posts for a given site.
    pub fn count(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
    ) -> Result<i64, SqliteDbError> {
        let sql = format!(
            "SELECT COUNT(*) FROM {} WHERE db_site_id = ?",
            Self::table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        stmt.query_row([site.row_id], |row| row.get(0))
            .map_err(SqliteDbError::from)
    }
}

// Context-specific implementations

impl PostContext for crate::context::EditContext {
    type Post = AnyPostWithEditContext;
    type DbPost = DbAnyPostWithEditContext;

    fn from_row_with_terms<F>(
        row: &Row,
        fetch_terms: F,
    ) -> Result<Self::DbPost, SqliteDbError>
    where
        F: FnOnce() -> Result<Vec<DbTermRelationship>, SqliteDbError>,
    {
        use PostEditContextColumn::*;

        let row_id: RowId = row.get_column(Rowid)?;
        let site = DbSite {
            row_id: row.get_column(PostEditContextColumn::SiteId)?,
        };

        // EditContext uses term relationships (categories and tags)
        let term_relationships = fetch_terms()?;
        let (categories, tags) = extract_categories_and_tags(term_relationships);

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

        Ok(DbAnyPostWithEditContext {
            row_id,
            site,
            post,
            last_fetched_at: row.get_column(LastFetchedAt)?,
        })
    }
}

impl PostContext for crate::context::ViewContext {
    type Post = AnyPostWithViewContext;
    type DbPost = DbAnyPostWithViewContext;

    fn from_row_with_terms<F>(
        row: &Row,
        fetch_terms: F,
    ) -> Result<Self::DbPost, SqliteDbError>
    where
        F: FnOnce() -> Result<Vec<DbTermRelationship>, SqliteDbError>,
    {
        use PostViewContextColumn::*;

        let row_id: RowId = row.get_column(Rowid)?;
        let site = DbSite {
            row_id: row.get_column(PostViewContextColumn::SiteId)?,
        };

        // ViewContext uses term relationships (categories and tags)
        let term_relationships = fetch_terms()?;
        let (categories, tags) = extract_categories_and_tags(term_relationships);

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

        Ok(DbAnyPostWithViewContext {
            row_id,
            site,
            post,
            last_fetched_at: row.get_column(LastFetchedAt)?,
        })
    }
}

impl PostContext for crate::context::EmbedContext {
    type Post = AnyPostWithEmbedContext;
    type DbPost = DbAnyPostWithEmbedContext;

    fn from_row_with_terms<F>(
        row: &Row,
        _fetch_terms: F,
    ) -> Result<Self::DbPost, SqliteDbError>
    where
        F: FnOnce() -> Result<Vec<DbTermRelationship>, SqliteDbError>,
    {
        use PostEmbedContextColumn::*;

        let row_id: RowId = row.get_column(Rowid)?;
        let site = DbSite {
            row_id: row.get_column(PostEmbedContextColumn::SiteId)?,
        };

        // EmbedContext does not use term relationships (no categories/tags in embed context)
        // The fetch_terms closure is never called, avoiding unnecessary database queries

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

        Ok(DbAnyPostWithEmbedContext {
            row_id,
            site,
            post,
            last_fetched_at: row.get_column(LastFetchedAt)?,
        })
    }
}

impl PostRepository<crate::context::EditContext> {
    /// Upsert a post with edit context and its term relationships (atomic transaction).
    ///
    /// Returns the rowid of the inserted or updated row.
    pub fn upsert(
        &self,
        transaction_manager: &mut impl TransactionManager,
        site: &DbSite,
        post: &wp_api::posts::AnyPostWithEditContext,
    ) -> Result<RowId, SqliteDbError> {
        let tx = transaction_manager.transaction()?;

        let upsert_sql = format!(
            r#"
            INSERT INTO {} (
                db_site_id, id, date, date_gmt, link, modified, modified_gmt, slug, status, post_type,
                password, template, permalink_template, generated_slug, author, featured_media,
                sticky, parent, menu_order, comment_status, ping_status, format, meta,
                guid_raw, guid_rendered, title_raw, title_rendered,
                content_raw, content_rendered, content_protected, content_block_version,
                excerpt_raw, excerpt_rendered, excerpt_protected
            ) VALUES (
                :db_site_id, :id, :date, :date_gmt, :link, :modified, :modified_gmt, :slug, :status, :post_type,
                :password, :template, :permalink_template, :generated_slug, :author, :featured_media,
                :sticky, :parent, :menu_order, :comment_status, :ping_status, :format, :meta,
                :guid_raw, :guid_rendered, :title_raw, :title_rendered,
                :content_raw, :content_rendered, :content_protected, :content_block_version,
                :excerpt_raw, :excerpt_rendered, :excerpt_protected
            )
            ON CONFLICT(db_site_id, id) DO UPDATE SET
                date = excluded.date,
                date_gmt = excluded.date_gmt,
                link = excluded.link,
                modified = excluded.modified,
                modified_gmt = excluded.modified_gmt,
                slug = excluded.slug,
                status = excluded.status,
                post_type = excluded.post_type,
                password = excluded.password,
                template = excluded.template,
                permalink_template = excluded.permalink_template,
                generated_slug = excluded.generated_slug,
                author = excluded.author,
                featured_media = excluded.featured_media,
                sticky = excluded.sticky,
                parent = excluded.parent,
                menu_order = excluded.menu_order,
                comment_status = excluded.comment_status,
                ping_status = excluded.ping_status,
                format = excluded.format,
                meta = excluded.meta,
                guid_raw = excluded.guid_raw,
                guid_rendered = excluded.guid_rendered,
                title_raw = excluded.title_raw,
                title_rendered = excluded.title_rendered,
                content_raw = excluded.content_raw,
                content_rendered = excluded.content_rendered,
                content_protected = excluded.content_protected,
                content_block_version = excluded.content_block_version,
                excerpt_raw = excluded.excerpt_raw,
                excerpt_rendered = excluded.excerpt_rendered,
                excerpt_protected = excluded.excerpt_protected,
                last_fetched_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            "#,
            Self::table_name()
        );

        tx.execute(
            &upsert_sql,
            rusqlite::named_params! {
                ":db_site_id": site.row_id,
                ":id": post.id.0,
                ":date": post.date,
                ":date_gmt": post.date_gmt.to_string(),
                ":link": post.link,
                ":modified": post.modified,
                ":modified_gmt": post.modified_gmt.to_string(),
                ":slug": post.slug,
                ":status": post.status.to_string(),
                ":post_type": post.post_type,
                ":password": post.password,
                ":template": post.template,
                ":permalink_template": post.permalink_template,
                ":generated_slug": post.generated_slug,
                ":author": post.author.map(|u| u.0),
                ":featured_media": post.featured_media.map(|m| m.0),
                ":sticky": bool_to_integer(post.sticky),
                ":parent": post.parent.map(|p| p.0),
                ":menu_order": post.menu_order,
                ":comment_status": post.comment_status.as_ref().map(|s| s.to_string()),
                ":ping_status": post.ping_status.as_ref().map(|s| s.to_string()),
                ":format": post.format.as_ref().map(|f| f.to_string()),
                ":meta": serialize_value_to_json(&post.meta)?,
                ":guid_raw": post.guid.raw,
                ":guid_rendered": post.guid.rendered,
                ":title_raw": post.title.raw,
                ":title_rendered": post.title.rendered,
                ":content_raw": post.content.raw,
                ":content_rendered": post.content.rendered,
                ":content_protected": post.content.protected,
                ":content_block_version": post.content.block_version,
                ":excerpt_raw": post.excerpt.as_ref().and_then(|e| e.raw.clone()),
                ":excerpt_rendered": post.excerpt.as_ref().and_then(|e| e.rendered.clone()),
                ":excerpt_protected": post.excerpt.as_ref().and_then(|e| e.protected),
            },
        )?;

        // Get the rowid of the upserted post
        let sql = format!(
            "SELECT rowid FROM {} WHERE db_site_id = ? AND id = ?",
            Self::table_name()
        );
        let post_rowid: i64 = {
            let mut stmt = tx.prepare(&sql)?;
            stmt.query_row(rusqlite::params![site.row_id, post.id.0], |row| row.get(0))
                .map_err(SqliteDbError::from)?
        };
        let post_rowid = RowId(post_rowid as u64);

        // Sync term relationships
        let term_repo = TermRelationshipRepository;

        if let Some(ref categories) = post.categories {
            term_repo.sync_terms_for_object(
                &tx,
                site,
                post.id.0,
                &TaxonomyType::Category,
                categories,
            )?;
        }

        if let Some(ref tags) = post.tags {
            term_repo.sync_terms_for_object(&tx, site, post.id.0, &TaxonomyType::PostTag, tags)?;
        }

        tx.commit().map_err(SqliteDbError::from)?;
        Ok(post_rowid)
    }

    /// Upsert multiple posts with their term relationships.
    pub fn upsert_batch(
        &self,
        transaction_manager: &mut impl TransactionManager,
        site: &DbSite,
        posts: &[wp_api::posts::AnyPostWithEditContext],
    ) -> Result<Vec<RowId>, SqliteDbError> {
        posts
            .iter()
            .map(|post| self.upsert(transaction_manager, site, post))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{
        TestContext, assert_recent_timestamp, posts::PostBuilder, test_ctx,
    };
    use rstest::*;
    use wp_api::posts::{AnyPostWithEditContext, PostStatus};

    #[rstest]
    #[case(PostBuilder::minimal().build())]
    #[case(PostBuilder::full().build())]
    fn test_round_trip(mut test_ctx: TestContext, #[case] original_post: AnyPostWithEditContext) {
        // Insert into database using repository
        let rowid = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &original_post)
            .expect("Failed to insert post");

        // Read back from database using PostRepository's select_by_rowid
        let retrieved = test_ctx
            .post_repo
            .select_by_rowid(&test_ctx.conn, &test_ctx.site, rowid)
            .expect("Failed to read post")
            .expect("Post should exist");

        // Verify round-trip
        assert_eq!(retrieved.row_id, rowid);
        assert_eq!(retrieved.site, test_ctx.site);
        assert_recent_timestamp(&retrieved.last_fetched_at);
        assert_eq!(retrieved.post, original_post);
    }

    #[rstest]
    #[case(PostStatus::Publish)]
    #[case(PostStatus::Draft)]
    #[case(PostStatus::Pending)]
    #[case(PostStatus::Private)]
    #[case(PostStatus::Future)]
    #[case(PostStatus::Custom("custom-status".to_string()))]
    fn test_round_trip_with_different_enum_variants(
        mut test_ctx: TestContext,
        #[case] post_status: PostStatus,
    ) {
        let post = PostBuilder::minimal()
            .with_status(post_status.clone())
            .build();

        let rowid = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();
        let retrieved = test_ctx
            .post_repo
            .select_by_rowid(&test_ctx.conn, &test_ctx.site, rowid)
            .unwrap()
            .expect("Post should exist");

        assert_eq!(retrieved.post.status, post_status);
    }

    #[rstest]
    fn test_round_trip_with_empty_json_arrays(mut test_ctx: TestContext) {
        let post = PostBuilder::minimal()
            .with_categories(vec![])
            .with_tags(vec![])
            .build();

        let rowid = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();
        let retrieved = test_ctx
            .post_repo
            .select_by_rowid(&test_ctx.conn, &test_ctx.site, rowid)
            .unwrap()
            .expect("Post should exist");

        assert_eq!(retrieved.post.categories, None);
        assert_eq!(retrieved.post.tags, None);
    }

    #[rstest]
    fn test_repository_insert_and_select_by_rowid(mut test_ctx: TestContext) {
        let post = PostBuilder::minimal().build();

        // Insert using repository
        let rowid = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .expect("Failed to insert");

        // Select by rowid
        let retrieved = test_ctx
            .post_repo
            .select_by_rowid(&test_ctx.conn, &test_ctx.site, rowid)
            .expect("Failed to select")
            .expect("Post should exist");

        assert_eq!(retrieved.row_id, rowid);
        assert_eq!(retrieved.site, test_ctx.site);
        assert_eq!(retrieved.post, post);
    }

    #[rstest]
    fn test_repository_select_by_post_id(mut test_ctx: TestContext) {
        let post = PostBuilder::minimal().with_id(42).build();

        // Insert
        test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .expect("Failed to insert");

        // Select by post_id
        let retrieved = test_ctx
            .post_repo
            .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(42))
            .expect("Failed to select by post_id")
            .expect("Post should exist");

        assert_eq!(retrieved.post.id, PostId(42));
        assert_eq!(retrieved.site, test_ctx.site);
        assert_eq!(retrieved.post, post);
    }

    #[rstest]
    fn test_repository_select_by_post_id_not_found(test_ctx: TestContext) {
        // Try to select non-existent post
        let result =
            test_ctx
                .post_repo
                .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(999));

        assert!(result.unwrap().is_none(), "Should return None when post doesn't exist");
    }

    #[rstest]
    fn test_repository_select_all(mut test_ctx: TestContext) {
        // Initially empty
        let all = test_ctx
            .post_repo
            .select_all(&test_ctx.conn, &test_ctx.site)
            .unwrap();
        assert_eq!(all.len(), 0);

        // Insert posts
        let post1 = PostBuilder::minimal().build();
        let post2 = PostBuilder::minimal().build();

        test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post1)
            .unwrap();
        test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post2)
            .unwrap();

        // Select all
        let all = test_ctx
            .post_repo
            .select_all(&test_ctx.conn, &test_ctx.site)
            .unwrap();
        assert_eq!(all.len(), 2);
    }

    #[rstest]
    fn test_repository_count(mut test_ctx: TestContext) {
        assert_eq!(
            test_ctx
                .post_repo
                .count(&test_ctx.conn, &test_ctx.site)
                .unwrap(),
            0
        );

        let post1 = PostBuilder::minimal().build();
        test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post1)
            .unwrap();

        assert_eq!(
            test_ctx
                .post_repo
                .count(&test_ctx.conn, &test_ctx.site)
                .unwrap(),
            1
        );

        let post2 = PostBuilder::minimal().build();
        test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post2)
            .unwrap();

        assert_eq!(
            test_ctx
                .post_repo
                .count(&test_ctx.conn, &test_ctx.site)
                .unwrap(),
            2
        );
    }

    #[rstest]
    fn test_repository_insert_batch(mut test_ctx: TestContext) {
        let post1 = PostBuilder::minimal().build();
        let post2 = PostBuilder::full().build();
        let post3 = PostBuilder::minimal().build();

        let posts = vec![post1, post2, post3];

        // Insert batch
        let rowids = test_ctx
            .post_repo
            .upsert_batch(&mut test_ctx.conn, &test_ctx.site, &posts)
            .unwrap();
        assert_eq!(rowids.len(), 3);

        // Verify all were inserted
        assert_eq!(
            test_ctx
                .post_repo
                .count(&test_ctx.conn, &test_ctx.site)
                .unwrap(),
            3
        );

        // Verify can retrieve each
        rowids.iter().for_each(|&rowid| {
            test_ctx
                .post_repo
                .select_by_rowid(&test_ctx.conn, &test_ctx.site, rowid)
                .expect("Should not error")
                .expect("Should exist");
        });
    }

    #[rstest]
    fn test_repository_delete_by_post_id(mut test_ctx: TestContext) {
        let post = PostBuilder::minimal().with_id(42).build();
        test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();

        // Verify exists
        test_ctx
            .post_repo
            .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(42))
            .expect("Should not error")
            .expect("Post should exist");

        // Delete
        let deleted = test_ctx
            .post_repo
            .delete_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(42))
            .unwrap();
        assert_eq!(deleted, 1);

        // Verify no longer exists
        let result =
            test_ctx
                .post_repo
                .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(42))
                .unwrap();
        assert!(result.is_none(), "Post should not exist after deletion");

        // Delete non-existent should return 0
        let deleted = test_ctx
            .post_repo
            .delete_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(999))
            .unwrap();
        assert_eq!(deleted, 0);
    }

    #[rstest]
    fn test_repository_upsert_inserts_new_post(mut test_ctx: TestContext) {
        let post = PostBuilder::minimal()
            .with_id(100)
            .with_status(PostStatus::Draft)
            .build();

        // Verify post doesn't exist
        assert!(
            test_ctx
                .post_repo
                .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(100))
                .unwrap()
                .is_none(),
            "Post should not exist before insert"
        );

        // Upsert should insert
        let rowid = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();

        // Verify it was inserted
        let retrieved = test_ctx
            .post_repo
            .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(100))
            .unwrap()
            .expect("Post should exist after insert");
        assert_eq!(retrieved.row_id, rowid);
        assert_eq!(retrieved.site, test_ctx.site);
        assert_eq!(retrieved.post.status, PostStatus::Draft);
    }

    #[rstest]
    fn test_repository_upsert_updates_existing_post(mut test_ctx: TestContext) {
        // Insert initial post
        let post = PostBuilder::minimal()
            .with_id(200)
            .with_status(PostStatus::Draft)
            .with_slug("original-slug")
            .build();

        let original_rowid = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();

        // Upsert with updated data
        let updated_post = PostBuilder::minimal()
            .with_id(200)
            .with_status(PostStatus::Publish)
            .with_slug("updated-slug")
            .build();

        let new_rowid = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &updated_post)
            .unwrap();

        // Rowid should be the same (it's an update, not delete+insert)
        assert_eq!(original_rowid, new_rowid);

        // Verify the update
        let retrieved = test_ctx
            .post_repo
            .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(200))
            .unwrap()
            .expect("Post should exist after update");
        assert_eq!(retrieved.post.status, PostStatus::Publish);
        assert_eq!(retrieved.post.slug, "updated-slug");

        // Verify only one post exists with this ID
        assert_eq!(
            test_ctx
                .post_repo
                .count(&test_ctx.conn, &test_ctx.site)
                .unwrap(),
            1
        );
    }

    #[rstest]
    fn test_upsert_inserts_post_and_terms(mut test_ctx: TestContext) {
        let post = PostBuilder::minimal()
            .with_id(300)
            .with_categories(vec![wp_api::terms::TermId(1), wp_api::terms::TermId(2)])
            .with_tags(vec![wp_api::terms::TermId(10), wp_api::terms::TermId(20)])
            .build();

        // Upsert with terms
        let rowid = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();

        // Verify post was inserted
        let retrieved = test_ctx
            .post_repo
            .select_by_rowid(&test_ctx.conn, &test_ctx.site, rowid)
            .unwrap()
            .expect("Post should exist");
        assert_eq!(retrieved.post.id, PostId(300));

        // Verify categories were inserted
        assert_eq!(retrieved.post.categories.as_ref().unwrap().len(), 2);
        assert!(
            retrieved
                .post
                .categories
                .as_ref()
                .unwrap()
                .contains(&wp_api::terms::TermId(1))
        );
        assert!(
            retrieved
                .post
                .categories
                .as_ref()
                .unwrap()
                .contains(&wp_api::terms::TermId(2))
        );

        // Verify tags were inserted
        assert_eq!(retrieved.post.tags.as_ref().unwrap().len(), 2);
        assert!(
            retrieved
                .post
                .tags
                .as_ref()
                .unwrap()
                .contains(&wp_api::terms::TermId(10))
        );
        assert!(
            retrieved
                .post
                .tags
                .as_ref()
                .unwrap()
                .contains(&wp_api::terms::TermId(20))
        );
    }

    #[rstest]
    fn test_upsert_updates_existing_terms(mut test_ctx: TestContext) {
        // Insert post with initial terms
        let post = PostBuilder::minimal()
            .with_id(400)
            .with_categories(vec![wp_api::terms::TermId(1), wp_api::terms::TermId(2)])
            .with_tags(vec![
                wp_api::terms::TermId(10),
                wp_api::terms::TermId(20),
                wp_api::terms::TermId(30),
            ])
            .build();

        test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();

        // Update with different terms
        let updated_post = PostBuilder::minimal()
            .with_id(400)
            .with_categories(vec![wp_api::terms::TermId(1), wp_api::terms::TermId(3)]) // Remove 2, add 3
            .with_tags(vec![wp_api::terms::TermId(10)]) // Remove 20, 30
            .build();

        test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &updated_post)
            .unwrap();

        // Verify updated terms
        let retrieved = test_ctx
            .post_repo
            .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(400))
            .unwrap()
            .expect("Post should exist");

        // Categories: should have 1, 3 (not 2)
        assert_eq!(retrieved.post.categories.as_ref().unwrap().len(), 2);
        assert!(
            retrieved
                .post
                .categories
                .as_ref()
                .unwrap()
                .contains(&wp_api::terms::TermId(1))
        );
        assert!(
            retrieved
                .post
                .categories
                .as_ref()
                .unwrap()
                .contains(&wp_api::terms::TermId(3))
        );
        assert!(
            !retrieved
                .post
                .categories
                .as_ref()
                .unwrap()
                .contains(&wp_api::terms::TermId(2))
        );

        // Tags: should only have 10 (not 20, 30)
        assert_eq!(retrieved.post.tags.as_ref().unwrap().len(), 1);
        assert_eq!(
            retrieved.post.tags.as_ref().unwrap()[0],
            wp_api::terms::TermId(10)
        );
    }

    #[rstest]
    fn test_delete_by_post_id_deletes_terms(mut test_ctx: TestContext) {
        // Insert post without terms (to avoid transaction issues in this test)
        let post = PostBuilder::minimal().with_id(500).build();
        test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();

        // Manually add terms using WordPress post ID
        let tx = test_ctx.conn.transaction().unwrap();
        test_ctx
            .term_repo
            .sync_terms_for_object(
                &tx,
                &test_ctx.site,
                post.id.0,
                &wp_api::taxonomies::TaxonomyType::Category,
                &[wp_api::terms::TermId(1), wp_api::terms::TermId(2)],
            )
            .unwrap();
        tx.commit().unwrap();

        // Verify terms exist
        let terms = test_ctx
            .term_repo
            .get_all_terms_for_object(&test_ctx.conn, &test_ctx.site, post.id.0)
            .unwrap();
        assert!(!terms.is_empty());

        // Delete post
        test_ctx
            .post_repo
            .delete_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(500))
            .unwrap();

        // Verify terms were also deleted
        let terms_after = test_ctx
            .term_repo
            .get_all_terms_for_object(&test_ctx.conn, &test_ctx.site, post.id.0)
            .unwrap();
        assert!(terms_after.is_empty());
    }

    #[rstest]
    fn test_select_by_rowid_populates_terms(mut test_ctx: TestContext) {
        // Insert post with terms
        let post = PostBuilder::minimal()
            .with_id(600)
            .with_categories(vec![wp_api::terms::TermId(5)])
            .build();

        let rowid = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();

        // Select by rowid should populate terms
        let retrieved = test_ctx
            .post_repo
            .select_by_rowid(&test_ctx.conn, &test_ctx.site, rowid)
            .unwrap()
            .expect("Post should exist");
        assert_eq!(
            retrieved.post.categories,
            Some(vec![wp_api::terms::TermId(5)])
        );
    }

    #[rstest]
    fn test_insert_sets_last_fetched_at(mut test_ctx: TestContext) {
        let post = PostBuilder::minimal().build();

        // Insert post
        let rowid = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();

        // Retrieve and validate last_fetched_at
        let retrieved = test_ctx
            .post_repo
            .select_by_rowid(&test_ctx.conn, &test_ctx.site, rowid)
            .unwrap()
            .expect("Post should exist");

        // Validate timestamp is recent and valid
        assert_recent_timestamp(&retrieved.last_fetched_at);
    }

    #[rstest]
    fn test_upsert_updates_last_fetched_at_on_update(mut test_ctx: TestContext) {
        let post = PostBuilder::minimal()
            .with_id(200)
            .with_title("Original Title")
            .build();

        // Initial insert
        test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();
        let first_fetch = test_ctx
            .post_repo
            .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(200))
            .unwrap()
            .expect("Post should exist")
            .last_fetched_at
            .clone();

        // Sleep a tiny bit to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Update post
        let updated_post = PostBuilder::minimal()
            .with_id(200)
            .with_title("Updated Title")
            .build();
        test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &updated_post)
            .unwrap();
        let second_fetch = test_ctx
            .post_repo
            .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(200))
            .unwrap()
            .expect("Post should exist")
            .last_fetched_at;

        // last_fetched_at should be updated (different)
        assert_ne!(first_fetch, second_fetch);

        // Both should be valid timestamps
        assert!(first_fetch.ends_with('Z'));
        assert!(second_fetch.ends_with('Z'));
    }
}
