use crate::{
    DbTable, RowId, SqliteDbError,
    context::{EditContext, EmbedContext, IsContext, ViewContext},
    db_types::{
        db_site::DbSite,
        helpers::{
            bool_to_integer, deserialize_json_value, get_date_string, get_id, get_optional_id,
            integer_to_bool, parse_datetime, parse_enum, parse_optional_enum,
            serialize_value_to_json,
        },
        posts::{
            DbAnyPostWithEditContext, DbAnyPostWithEmbedContext, DbAnyPostWithViewContext,
            PostEditContextColumn, PostEmbedContextColumn, PostViewContextColumn,
        },
        row_ext::RowExt,
    },
    entity::{EntityId, FullEntity},
    repository::{
        QueryExecutor, TransactionManager, term_relationships::TermRelationshipRepository,
    },
    term_relationships::DbTermRelationship,
};
use rusqlite::{OptionalExtension, Row};
use std::{collections::HashMap, marker::PhantomData, sync::Arc};
use wp_api::{
    WpAdditionalFields,
    posts::{
        AnyPostWithEditContext, AnyPostWithEmbedContext, AnyPostWithViewContext,
        PostContentWithEditContext, PostContentWithViewContext, PostGuidWithEditContext,
        PostGuidWithViewContext, PostId, PostTitleWithEditContext, PostTitleWithEmbedContext,
        PostTitleWithViewContext, SparsePostExcerpt,
    },
    prelude::WpGmtDateTime,
    taxonomies::TaxonomyType,
    terms::TermId,
};

/// Entity-specific context trait for Posts.
///
/// Associates a context with post-specific types and provides database row mapping.
pub trait PostContext: IsContext {
    /// The context-specific post entity type (e.g., AnyPostWithEditContext)
    type Post;

    /// The context-specific database wrapper type (e.g., DbAnyPostWithEditContext)
    type DbPost;

    /// Get the database table for this context
    fn table() -> DbTable;

    /// Construct DbPost from a database row with lazy term relationship loading.
    ///
    /// The `fetch_terms` closure is only called if the context actually needs term relationships.
    /// This allows contexts like Embed (which don't use terms) to avoid unnecessary database queries.
    fn from_row_with_terms<F>(row: &Row, fetch_terms: F) -> Result<Self::DbPost, SqliteDbError>
    where
        F: FnOnce() -> Result<Vec<DbTermRelationship>, SqliteDbError>;

    /// Extract the rowid from DbPost (for EntityId creation)
    fn rowid(db_post: &Self::DbPost) -> RowId;
}

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
    /// Create a new repository instance.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }

    /// Get the full table name for this context.
    pub fn table_name() -> &'static str {
        C::table().table_name()
    }

    /// Select a post by its EntityId (returns wrapper with rowid).
    ///
    /// Returns an error if the EntityId's table name doesn't match this repository's context.
    /// Returns `Ok(None)` if no post with the given EntityId exists.
    /// Automatically populates categories and tags from term_relationships table.
    pub fn select_by_entity_id(
        &self,
        executor: &impl QueryExecutor,
        entity_id: &EntityId,
    ) -> Result<Option<FullEntity<C::DbPost>>, SqliteDbError> {
        // Validate that the entity_id is for the correct table
        entity_id.validate_table(C::table())?;

        // First get the post.id (WordPress ID) from the rowid
        let sql = format!(
            "SELECT id FROM {} WHERE db_site_id = ? AND rowid = ?",
            Self::table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        let Some(post_id) = stmt
            .query_row([entity_id.db_site.row_id, entity_id.rowid], |row| {
                row.get(0)
            })
            .optional()
            .map_err(SqliteDbError::from)?
        else {
            return Ok(None);
        };

        // Pre-load term relationships for this post
        let term_repo = TermRelationshipRepository;
        let terms_map =
            term_repo.get_terms_for_objects(executor, &entity_id.db_site, &[post_id])?;

        // Query and construct post with pre-loaded term relationships
        let sql = format!(
            "SELECT * FROM {} WHERE db_site_id = ? AND rowid = ?",
            Self::table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        let db_post = stmt
            .query_row([entity_id.db_site.row_id, entity_id.rowid], |row| {
                C::from_row_with_terms(row, || {
                    Ok(terms_map.get(&post_id).cloned().unwrap_or_default())
                })
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
            })
            .optional()
            .map_err(SqliteDbError::from)?;

        Ok(db_post.map(|db_post| {
            let entity_id = Arc::new(*entity_id);
            FullEntity::new(entity_id, db_post)
        }))
    }

    /// Select all posts for a given site (returns wrappers with rowids).
    ///
    /// Returns an empty vector if no posts exist for the site.
    /// Automatically populates categories and tags from term_relationships table.
    pub fn select_all(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
    ) -> Result<Vec<FullEntity<C::DbPost>>, SqliteDbError> {
        self.select_by_filter(executor, site, None)
    }

    /// Select posts filtered by criteria.
    ///
    /// Similar to `select_all` but applies filtering based on provided parameters.
    /// Currently supports filtering by status. More filters can be added as needed.
    ///
    /// # Arguments
    /// * `executor` - Database connection or transaction
    /// * `site` - The site to query posts from
    /// * `status` - Optional post status filter (e.g., "publish", "draft")
    ///
    /// # Returns
    /// Vector of posts matching the filter criteria, empty if no matches found.
    pub fn select_by_filter(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        status: Option<&wp_api::posts::PostStatus>,
    ) -> Result<Vec<FullEntity<C::DbPost>>, SqliteDbError> {
        // Build WHERE clause
        let mut where_clauses = vec!["db_site_id = ?"];
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(site.row_id)];

        if let Some(status_value) = status {
            where_clauses.push("status = ?");
            params.push(Box::new(status_value.to_string()));
        }

        let where_clause = where_clauses.join(" AND ");

        // First pass: extract post IDs (WordPress IDs, not SQLite rowids)
        let sql = format!(
            "SELECT id FROM {} WHERE {}",
            Self::table_name(),
            where_clause
        );
        let mut stmt = executor.prepare(&sql)?;
        let post_ids: Vec<i64> = stmt
            .query_map(
                rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
                |row| row.get(0),
            )?
            .collect::<Result<Vec<_>, _>>()
            .map_err(SqliteDbError::from)?;

        if post_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Batch load term relationships for all posts using WordPress post IDs
        let term_repo = TermRelationshipRepository;
        let terms_map = term_repo.get_terms_for_objects(executor, site, &post_ids)?;

        // Rebuild params for second query (need fresh boxes since params were consumed)
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(site.row_id)];
        if let Some(status_value) = status {
            params.push(Box::new(status_value.to_string()));
        }

        // Second pass: construct posts with lazy term relationship access
        let sql = format!(
            "SELECT * FROM {} WHERE {}",
            Self::table_name(),
            where_clause
        );
        let mut stmt = executor.prepare(&sql)?;
        let posts = stmt
            .query_map(
                rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
                |row| {
                    let post_id: i64 = row.get("id")?;
                    C::from_row_with_terms(row, || {
                        Ok(terms_map.get(&post_id).cloned().unwrap_or_default())
                    })
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
                },
            )?
            .collect::<Result<Vec<_>, _>>()
            .map_err(SqliteDbError::from)?;

        Ok(posts
            .into_iter()
            .map(|db_post| {
                let rowid = C::rowid(&db_post);
                let entity_id = Arc::new(EntityId::new(*site, C::table(), rowid));
                FullEntity::new(entity_id, db_post)
            })
            .collect())
    }

    /// Select a post by its WordPress post ID for a given site.
    ///
    /// Returns the post data paired with its EntityId, which encapsulates the
    /// database identity (site_id, table_name, rowid).
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
    ) -> Result<Option<FullEntity<C::DbPost>>, SqliteDbError> {
        // Pre-load term relationships for this post
        let term_repo = TermRelationshipRepository;
        let terms_map = term_repo.get_terms_for_objects(executor, site, &[post_id.0])?;

        // Query and construct post with pre-loaded term relationships
        let sql = format!(
            "SELECT * FROM {} WHERE db_site_id = ? AND id = ?",
            Self::table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        let db_post = stmt
            .query_row(rusqlite::params![site.row_id, post_id.0], |row| {
                C::from_row_with_terms(row, || {
                    Ok(terms_map.get(&post_id.0).cloned().unwrap_or_default())
                })
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
            })
            .optional()
            .map_err(SqliteDbError::from)?;

        // Wrap in FullEntity with EntityId
        Ok(db_post.map(|db_post| {
            let rowid = C::rowid(&db_post);

            let entity_id = Arc::new(EntityId::new(*site, C::table(), rowid));

            FullEntity::new(entity_id, db_post)
        }))
    }

    /// Select `modified_gmt` timestamps for multiple posts by their WordPress post IDs.
    ///
    /// This is a lightweight query used for staleness detection - it only fetches
    /// the `id` and `modified_gmt` columns without loading the full post data.
    ///
    /// Posts not found in the cache are omitted from the result. A cached post
    /// whose `modified_gmt` is absent or unreadable maps to `None`, so a caller
    /// can tell it apart from one that isn't cached and decide for itself
    /// rather than being handed silence.
    ///
    /// # Arguments
    /// * `executor` - Database connection or transaction
    /// * `site` - The site to query posts for
    /// * `post_ids` - WordPress post IDs to look up
    pub fn select_modified_gmt_by_ids(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        post_ids: &[PostId],
    ) -> Result<HashMap<PostId, Option<WpGmtDateTime>>, SqliteDbError> {
        if post_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let ids_str = post_ids
            .iter()
            .map(|id| id.0.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            "SELECT id, modified_gmt FROM {} WHERE db_site_id = ? AND id IN ({})",
            Self::table_name(),
            ids_str
        );

        let mut stmt = executor.prepare(&sql)?;
        let rows = stmt.query_map([site.row_id], |row| {
            let id: i64 = row.get(0)?;
            let modified_gmt_str: Option<String> = row.get(1)?;
            Ok((id, modified_gmt_str))
        })?;

        Ok(rows
            .filter_map(|row_result| {
                let (id, modified_gmt_str) = row_result.ok()?;
                let modified_gmt = modified_gmt_str.and_then(|s| s.parse::<WpGmtDateTime>().ok());
                Some((PostId(id), modified_gmt))
            })
            .collect())
    }

    /// Delete a post by its EntityId for a given site.
    ///
    /// Returns the number of rows deleted (0 or 1).
    /// Automatically deletes associated term relationships.
    ///
    /// Returns an error if the EntityId's table name doesn't match this repository's context.
    /// Returns `Ok(0)` if no post with the given EntityId exists.
    pub fn delete_by_entity_id(
        &self,
        executor: &impl QueryExecutor,
        entity_id: &EntityId,
    ) -> Result<usize, SqliteDbError> {
        // Validate that the entity_id is for the correct table
        entity_id.validate_table(C::table())?;

        // Get the WordPress post ID from the rowid (lightweight SELECT)
        let sql = format!(
            "SELECT id FROM {} WHERE db_site_id = ? AND rowid = ?",
            Self::table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        let post_id = stmt
            .query_row([entity_id.db_site.row_id, entity_id.rowid], |row| {
                row.get::<_, i64>(0)
            })
            .optional()
            .map_err(SqliteDbError::from)?;

        match post_id {
            Some(id) => self.delete_by_post_id(executor, &entity_id.db_site, PostId(id)),
            None => Ok(0), // Post doesn't exist
        }
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

impl PostContext for EditContext {
    type Post = AnyPostWithEditContext;
    type DbPost = DbAnyPostWithEditContext;

    fn table() -> DbTable {
        DbTable::PostsEditContext
    }

    fn from_row_with_terms<F>(row: &Row, fetch_terms: F) -> Result<Self::DbPost, SqliteDbError>
    where
        F: FnOnce() -> Result<Vec<DbTermRelationship>, SqliteDbError>,
    {
        use PostEditContextColumn::*;

        let row_id: RowId = row.get_column(Rowid)?;
        let db_site_id: RowId = row.get_column(PostEditContextColumn::DbSiteId)?;

        // EditContext uses term relationships (categories and tags)
        let term_relationships = fetch_terms()?;
        let (categories, tags) = extract_categories_and_tags(term_relationships);

        let post = AnyPostWithEditContext {
            id: get_id(row, Id)?,
            date: get_date_string(row, Date)?,
            date_gmt: parse_datetime(row, DateGmt)?,
            guid: PostGuidWithEditContext {
                raw: row.get_column(GuidRaw)?,
                rendered: row.get_column(GuidRendered)?,
            },
            link: row.get_column(Link)?,
            modified: get_date_string(row, Modified)?,
            modified_gmt: parse_datetime(row, ModifiedGmt)?,
            slug: row.get_column(Slug)?,
            status: parse_enum(row, Status)?,
            post_type: row.get_column(PostType)?,
            password: row.get_column(Password)?,
            permalink_template: row.get_column(PermalinkTemplate)?,
            generated_slug: row.get_column(GeneratedSlug)?,
            title: {
                let title_rendered: Option<String> = row.get_column(TitleRendered)?;
                title_rendered.map(|rendered| PostTitleWithEditContext {
                    raw: row.get_column(TitleRaw).ok().flatten(),
                    rendered,
                })
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
            additional_fields: deserialize_json_value::<WpAdditionalFields>(
                row.get_column(AdditionalFields)?,
            )?
            .map(Arc::new),
        };

        Ok(DbAnyPostWithEditContext {
            row_id,
            db_site_id,
            post,
            last_fetched_at: row.get_column(LastFetchedAt)?,
        })
    }

    fn rowid(db_post: &Self::DbPost) -> RowId {
        db_post.row_id
    }
}

impl PostContext for ViewContext {
    type Post = AnyPostWithViewContext;
    type DbPost = DbAnyPostWithViewContext;

    fn table() -> DbTable {
        DbTable::PostsViewContext
    }

    fn from_row_with_terms<F>(row: &Row, fetch_terms: F) -> Result<Self::DbPost, SqliteDbError>
    where
        F: FnOnce() -> Result<Vec<DbTermRelationship>, SqliteDbError>,
    {
        use PostViewContextColumn::*;

        let row_id: RowId = row.get_column(Rowid)?;
        let db_site_id: RowId = row.get_column(PostViewContextColumn::DbSiteId)?;

        // ViewContext uses term relationships (categories and tags)
        let term_relationships = fetch_terms()?;
        let (categories, tags) = extract_categories_and_tags(term_relationships);

        let post = AnyPostWithViewContext {
            id: get_id(row, Id)?,
            date: get_date_string(row, Date)?,
            date_gmt: parse_datetime(row, DateGmt)?,
            guid: PostGuidWithViewContext {
                rendered: row.get_column(GuidRendered)?,
            },
            link: row.get_column(Link)?,
            modified: get_date_string(row, Modified)?,
            modified_gmt: parse_datetime(row, ModifiedGmt)?,
            slug: row.get_column(Slug)?,
            status: parse_enum(row, Status)?,
            post_type: row.get_column(PostType)?,
            title: {
                let title_rendered: Option<String> = row.get_column(TitleRendered)?;
                title_rendered.map(|rendered| PostTitleWithViewContext { rendered })
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
            additional_fields: deserialize_json_value::<WpAdditionalFields>(
                row.get_column(AdditionalFields)?,
            )?
            .map(Arc::new),
        };

        Ok(DbAnyPostWithViewContext {
            row_id,
            db_site_id,
            post,
            last_fetched_at: row.get_column(LastFetchedAt)?,
        })
    }

    fn rowid(db_post: &Self::DbPost) -> RowId {
        db_post.row_id
    }
}

impl PostContext for EmbedContext {
    type Post = AnyPostWithEmbedContext;
    type DbPost = DbAnyPostWithEmbedContext;

    fn table() -> DbTable {
        DbTable::PostsEmbedContext
    }

    fn from_row_with_terms<F>(row: &Row, _fetch_terms: F) -> Result<Self::DbPost, SqliteDbError>
    where
        F: FnOnce() -> Result<Vec<DbTermRelationship>, SqliteDbError>,
    {
        use PostEmbedContextColumn::*;

        let row_id: RowId = row.get_column(Rowid)?;
        let db_site_id: RowId = row.get_column(PostEmbedContextColumn::DbSiteId)?;

        // EmbedContext does not use term relationships (no categories/tags in embed context)
        // The fetch_terms closure is never called, avoiding unnecessary database queries

        let post = AnyPostWithEmbedContext {
            id: get_id(row, Id)?,
            date: get_date_string(row, Date)?,
            link: row.get_column(Link)?,
            slug: row.get_column(Slug)?,
            post_type: row.get_column(PostType)?,
            title: Some(PostTitleWithEmbedContext {
                rendered: row.get_column(TitleRendered)?,
            }),
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
            additional_fields: deserialize_json_value::<WpAdditionalFields>(
                row.get_column(AdditionalFields)?,
            )?
            .map(Arc::new),
        };

        Ok(DbAnyPostWithEmbedContext {
            row_id,
            db_site_id,
            post,
            last_fetched_at: row.get_column(LastFetchedAt)?,
        })
    }

    fn rowid(db_post: &Self::DbPost) -> RowId {
        db_post.row_id
    }
}

impl PostRepository<EditContext> {
    /// Upsert a post with edit context and its term relationships (atomic transaction).
    ///
    /// Returns the EntityId of the inserted or updated row.
    pub fn upsert(
        &self,
        transaction_manager: &mut impl TransactionManager,
        site: &DbSite,
        post: &AnyPostWithEditContext,
    ) -> Result<EntityId, SqliteDbError> {
        let tx = transaction_manager.transaction()?;

        let upsert_sql = format!(
            r#"
            INSERT INTO {} (
                db_site_id, id, date, date_gmt, link, modified, modified_gmt, slug, status, post_type,
                password, template, permalink_template, generated_slug, author, featured_media,
                sticky, parent, menu_order, comment_status, ping_status, format, meta,
                guid_raw, guid_rendered, title_raw, title_rendered,
                content_raw, content_rendered, content_protected, content_block_version,
                excerpt_raw, excerpt_rendered, excerpt_protected,
                additional_fields
            ) VALUES (
                :db_site_id, :id, :date, :date_gmt, :link, :modified, :modified_gmt, :slug, :status, :post_type,
                :password, :template, :permalink_template, :generated_slug, :author, :featured_media,
                :sticky, :parent, :menu_order, :comment_status, :ping_status, :format, :meta,
                :guid_raw, :guid_rendered, :title_raw, :title_rendered,
                :content_raw, :content_rendered, :content_protected, :content_block_version,
                :excerpt_raw, :excerpt_rendered, :excerpt_protected,
                :additional_fields
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
                additional_fields = excluded.additional_fields,
                last_fetched_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            RETURNING rowid
            "#,
            Self::table_name()
        );

        let post_rowid: i64 = tx
            .query_row(
                &upsert_sql,
                rusqlite::named_params! {
                    ":db_site_id": site.row_id,
                    ":id": post.id.0,
                    ":date": post.date.0,
                    ":date_gmt": post.date_gmt.to_string(),
                    ":link": post.link,
                    ":modified": post.modified.0,
                    ":modified_gmt": post.modified_gmt.to_string(),
                    ":slug": post.slug,
                    ":status": post.status.to_string(),
                    ":post_type": post.post_type,
                    ":password": post.password.clone(),
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
                    ":title_raw": post.title.as_ref().and_then(|t| t.raw.clone()),
                    ":title_rendered": post.title.as_ref().map(|t| t.rendered.clone()),
                    ":content_raw": post.content.raw,
                    ":content_rendered": post.content.rendered,
                    ":content_protected": post.content.protected,
                    ":content_block_version": post.content.block_version,
                    ":excerpt_raw": post.excerpt.as_ref().and_then(|e| e.raw.clone()),
                    ":excerpt_rendered": post.excerpt.as_ref().and_then(|e| e.rendered.clone()),
                    ":excerpt_protected": post.excerpt.as_ref().and_then(|e| e.protected),
                    ":additional_fields": serialize_value_to_json(&post.additional_fields)?,
                },
                |row| row.get(0),
            )
            .map_err(SqliteDbError::from)?;
        let post_rowid = RowId(post_rowid);

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
        Ok(EntityId::new(*site, EditContext::table(), post_rowid))
    }

    /// Upsert multiple posts with their term relationships.
    pub fn upsert_batch(
        &self,
        transaction_manager: &mut impl TransactionManager,
        site: &DbSite,
        posts: &[AnyPostWithEditContext],
    ) -> Result<Vec<EntityId>, SqliteDbError> {
        posts
            .iter()
            .map(|post| self.upsert(transaction_manager, site, post))
            .collect()
    }
}

impl PostRepository<ViewContext> {
    /// Upsert a post with view context and its term relationships (atomic transaction).
    ///
    /// Returns the EntityId of the inserted or updated row.
    pub fn upsert(
        &self,
        transaction_manager: &mut impl TransactionManager,
        site: &DbSite,
        post: &AnyPostWithViewContext,
    ) -> Result<EntityId, SqliteDbError> {
        let tx = transaction_manager.transaction()?;

        let upsert_sql = format!(
            r#"
            INSERT INTO {} (
                db_site_id, id, date, date_gmt, link, modified, modified_gmt, slug, status, post_type,
                template, author, featured_media, sticky, parent, menu_order,
                comment_status, ping_status, format, meta,
                guid_rendered, title_rendered,
                content_rendered, content_protected,
                excerpt_raw, excerpt_rendered, excerpt_protected,
                additional_fields
            ) VALUES (
                :db_site_id, :id, :date, :date_gmt, :link, :modified, :modified_gmt, :slug, :status, :post_type,
                :template, :author, :featured_media, :sticky, :parent, :menu_order,
                :comment_status, :ping_status, :format, :meta,
                :guid_rendered, :title_rendered,
                :content_rendered, :content_protected,
                :excerpt_raw, :excerpt_rendered, :excerpt_protected,
                :additional_fields
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
                template = excluded.template,
                author = excluded.author,
                featured_media = excluded.featured_media,
                sticky = excluded.sticky,
                parent = excluded.parent,
                menu_order = excluded.menu_order,
                comment_status = excluded.comment_status,
                ping_status = excluded.ping_status,
                format = excluded.format,
                meta = excluded.meta,
                guid_rendered = excluded.guid_rendered,
                title_rendered = excluded.title_rendered,
                content_rendered = excluded.content_rendered,
                content_protected = excluded.content_protected,
                excerpt_raw = excluded.excerpt_raw,
                excerpt_rendered = excluded.excerpt_rendered,
                excerpt_protected = excluded.excerpt_protected,
                additional_fields = excluded.additional_fields,
                last_fetched_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            RETURNING rowid
            "#,
            Self::table_name()
        );

        let post_rowid: i64 = tx
            .query_row(
                &upsert_sql,
                rusqlite::named_params! {
                    ":db_site_id": site.row_id,
                    ":id": post.id.0,
                    ":date": post.date.0,
                    ":date_gmt": post.date_gmt.to_string(),
                    ":link": post.link,
                    ":modified": post.modified.0,
                    ":modified_gmt": post.modified_gmt.to_string(),
                    ":slug": post.slug,
                    ":status": post.status.to_string(),
                    ":post_type": post.post_type,
                    ":template": post.template,
                    ":author": post.author.map(|u| u.0),
                    ":featured_media": post.featured_media.map(|m| m.0),
                    ":sticky": bool_to_integer(post.sticky),
                    ":parent": post.parent.map(|p| p.0),
                    ":menu_order": post.menu_order,
                    ":comment_status": post.comment_status.as_ref().map(|s| s.to_string()),
                    ":ping_status": post.ping_status.as_ref().map(|s| s.to_string()),
                    ":format": post.format.as_ref().map(|f| f.to_string()),
                    ":meta": serialize_value_to_json(&post.meta)?,
                    ":guid_rendered": post.guid.rendered,
                    ":title_rendered": post.title.as_ref().map(|t| t.rendered.clone()),
                    ":content_rendered": post.content.rendered,
                    ":content_protected": post.content.protected,
                    ":excerpt_raw": post.excerpt.as_ref().and_then(|e| e.raw.clone()),
                    ":excerpt_rendered": post.excerpt.as_ref().and_then(|e| e.rendered.clone()),
                    ":excerpt_protected": post.excerpt.as_ref().and_then(|e| e.protected),
                    ":additional_fields": serialize_value_to_json(&post.additional_fields)?,
                },
                |row| row.get(0),
            )
            .map_err(SqliteDbError::from)?;
        let post_rowid = RowId(post_rowid);

        // Sync term relationships (ViewContext has categories and tags)
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
        Ok(EntityId::new(*site, ViewContext::table(), post_rowid))
    }

    /// Upsert multiple posts with their term relationships.
    pub fn upsert_batch(
        &self,
        transaction_manager: &mut impl TransactionManager,
        site: &DbSite,
        posts: &[AnyPostWithViewContext],
    ) -> Result<Vec<EntityId>, SqliteDbError> {
        posts
            .iter()
            .map(|post| self.upsert(transaction_manager, site, post))
            .collect()
    }
}

impl PostRepository<EmbedContext> {
    /// Upsert a post with embed context (atomic transaction).
    ///
    /// Note: EmbedContext does not include categories or tags, so no term relationships are synced.
    ///
    /// Returns the EntityId of the inserted or updated row.
    pub fn upsert(
        &self,
        transaction_manager: &mut impl TransactionManager,
        site: &DbSite,
        post: &AnyPostWithEmbedContext,
    ) -> Result<EntityId, SqliteDbError> {
        let tx = transaction_manager.transaction()?;

        let upsert_sql = format!(
            r#"
            INSERT INTO {} (
                db_site_id, id, date, link, slug, post_type,
                title_rendered, author,
                excerpt_raw, excerpt_rendered, excerpt_protected,
                featured_media,
                additional_fields
            ) VALUES (
                :db_site_id, :id, :date, :link, :slug, :post_type,
                :title_rendered, :author,
                :excerpt_raw, :excerpt_rendered, :excerpt_protected,
                :featured_media,
                :additional_fields
            )
            ON CONFLICT(db_site_id, id) DO UPDATE SET
                date = excluded.date,
                link = excluded.link,
                slug = excluded.slug,
                post_type = excluded.post_type,
                title_rendered = excluded.title_rendered,
                author = excluded.author,
                excerpt_raw = excluded.excerpt_raw,
                excerpt_rendered = excluded.excerpt_rendered,
                excerpt_protected = excluded.excerpt_protected,
                featured_media = excluded.featured_media,
                additional_fields = excluded.additional_fields,
                last_fetched_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            RETURNING rowid
            "#,
            Self::table_name()
        );

        let post_rowid: i64 = tx
            .query_row(
                &upsert_sql,
                rusqlite::named_params! {
                    ":db_site_id": site.row_id,
                    ":id": post.id.0,
                    ":date": post.date.0,
                    ":link": post.link,
                    ":slug": post.slug,
                    ":post_type": post.post_type,
                    ":title_rendered": post.title.as_ref().map(|t| t.rendered.clone()),
                    ":author": post.author.map(|u| u.0),
                    ":excerpt_raw": post.excerpt.as_ref().and_then(|e| e.raw.clone()),
                    ":excerpt_rendered": post.excerpt.as_ref().and_then(|e| e.rendered.clone()),
                    ":excerpt_protected": post.excerpt.as_ref().and_then(|e| e.protected),
                    ":featured_media": post.featured_media.map(|m| m.0),
                    ":additional_fields": serialize_value_to_json(&post.additional_fields)?,
                },
                |row| row.get(0),
            )
            .map_err(SqliteDbError::from)?;
        let post_rowid = RowId(post_rowid);

        // No term relationships for EmbedContext (no categories or tags)

        tx.commit().map_err(SqliteDbError::from)?;
        Ok(EntityId::new(*site, EmbedContext::table(), post_rowid))
    }

    /// Upsert multiple posts.
    pub fn upsert_batch(
        &self,
        transaction_manager: &mut impl TransactionManager,
        site: &DbSite,
        posts: &[AnyPostWithEmbedContext],
    ) -> Result<Vec<EntityId>, SqliteDbError> {
        posts
            .iter()
            .map(|post| self.upsert(transaction_manager, site, post))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_types::posts::{
        PostEditContextColumn, PostEmbedContextColumn, PostViewContextColumn,
    };
    use crate::db_types::row_ext::ColumnIndex;
    use crate::test_fixtures::{
        TestContext, assert_recent_timestamp, get_table_column_names, posts::PostBuilder, test_ctx,
    };
    use rstest::*;
    use wp_api::posts::{AnyPostWithEditContext, PostStatus};

    /// A cached `modified_gmt` that can't be read maps to `None` instead of
    /// dropping the row, so a caller can tell "cached but unreadable" apart
    /// from "not cached at all".
    ///
    /// Dropping it made the staleness check treat the post as current, because
    /// a missing entry there means "not stale" — so the row would never be
    /// refetched and the stale copy stayed in the cache indefinitely.
    #[rstest]
    #[case::never_set_date("'-0001-11-30T00:00:00'")]
    #[case::unparseable("'not a date'")]
    fn test_unreadable_cached_modified_gmt_maps_to_none(
        mut test_ctx: TestContext,
        #[case] stored_value: &str,
    ) {
        let post = PostBuilder::minimal().build();
        let post_id = post.id;
        test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .expect("Failed to insert post");

        test_ctx
            .conn
            .execute(
                &format!("UPDATE posts_edit_context SET modified_gmt = {stored_value}"),
                [],
            )
            .expect("Failed to overwrite the cached timestamp");

        let cached = test_ctx
            .post_repo
            .select_modified_gmt_by_ids(&test_ctx.conn, &test_ctx.site, &[post_id])
            .expect("Failed to select cached timestamps");

        assert_eq!(
            cached.get(&post_id),
            Some(&None),
            "an unreadable timestamp should be present as None, not missing"
        );
    }

    /// Verify that PostEditContextColumn enum values match the actual database schema.
    /// This test protects against column reordering in migrations breaking the positional index mapping.
    #[rstest]
    fn test_post_edit_context_column_enum_matches_schema(test_ctx: TestContext) {
        use PostEditContextColumn::*;

        let columns = get_table_column_names(&test_ctx.conn, "posts_edit_context");

        // Verify each enum value maps to the correct column name
        assert_eq!(columns[Rowid.as_index()], "rowid");
        assert_eq!(columns[DbSiteId.as_index()], "db_site_id");
        assert_eq!(columns[Id.as_index()], "id");
        assert_eq!(columns[Date.as_index()], "date");
        assert_eq!(columns[DateGmt.as_index()], "date_gmt");
        assert_eq!(columns[Link.as_index()], "link");
        assert_eq!(columns[Modified.as_index()], "modified");
        assert_eq!(columns[ModifiedGmt.as_index()], "modified_gmt");
        assert_eq!(columns[Slug.as_index()], "slug");
        assert_eq!(columns[Status.as_index()], "status");
        assert_eq!(columns[PostType.as_index()], "post_type");
        assert_eq!(columns[Password.as_index()], "password");
        assert_eq!(columns[Template.as_index()], "template");
        assert_eq!(columns[PermalinkTemplate.as_index()], "permalink_template");
        assert_eq!(columns[GeneratedSlug.as_index()], "generated_slug");
        assert_eq!(columns[Author.as_index()], "author");
        assert_eq!(columns[FeaturedMedia.as_index()], "featured_media");
        assert_eq!(columns[Sticky.as_index()], "sticky");
        assert_eq!(columns[Parent.as_index()], "parent");
        assert_eq!(columns[MenuOrder.as_index()], "menu_order");
        assert_eq!(columns[CommentStatus.as_index()], "comment_status");
        assert_eq!(columns[PingStatus.as_index()], "ping_status");
        assert_eq!(columns[Format.as_index()], "format");
        assert_eq!(columns[Meta.as_index()], "meta");
        assert_eq!(columns[GuidRaw.as_index()], "guid_raw");
        assert_eq!(columns[GuidRendered.as_index()], "guid_rendered");
        assert_eq!(columns[TitleRaw.as_index()], "title_raw");
        assert_eq!(columns[TitleRendered.as_index()], "title_rendered");
        assert_eq!(columns[ContentRaw.as_index()], "content_raw");
        assert_eq!(columns[ContentRendered.as_index()], "content_rendered");
        assert_eq!(columns[ContentProtected.as_index()], "content_protected");
        assert_eq!(
            columns[ContentBlockVersion.as_index()],
            "content_block_version"
        );
        assert_eq!(columns[ExcerptRaw.as_index()], "excerpt_raw");
        assert_eq!(columns[ExcerptRendered.as_index()], "excerpt_rendered");
        assert_eq!(columns[ExcerptProtected.as_index()], "excerpt_protected");
        assert_eq!(columns[LastFetchedAt.as_index()], "last_fetched_at");
        assert_eq!(columns[AdditionalFields.as_index()], "additional_fields");

        // Verify total column count matches
        assert_eq!(columns.len(), AdditionalFields.as_index() + 1);
    }

    /// Verify that PostViewContextColumn enum values match the actual database schema.
    /// This test protects against column reordering in migrations breaking the positional index mapping.
    #[rstest]
    fn test_post_view_context_column_enum_matches_schema(test_ctx: TestContext) {
        use PostViewContextColumn::*;

        let columns = get_table_column_names(&test_ctx.conn, "posts_view_context");

        assert_eq!(columns[Rowid.as_index()], "rowid");
        assert_eq!(columns[DbSiteId.as_index()], "db_site_id");
        assert_eq!(columns[Id.as_index()], "id");
        assert_eq!(columns[Date.as_index()], "date");
        assert_eq!(columns[DateGmt.as_index()], "date_gmt");
        assert_eq!(columns[Link.as_index()], "link");
        assert_eq!(columns[Modified.as_index()], "modified");
        assert_eq!(columns[ModifiedGmt.as_index()], "modified_gmt");
        assert_eq!(columns[Slug.as_index()], "slug");
        assert_eq!(columns[Status.as_index()], "status");
        assert_eq!(columns[PostType.as_index()], "post_type");
        assert_eq!(columns[Template.as_index()], "template");
        assert_eq!(columns[Author.as_index()], "author");
        assert_eq!(columns[FeaturedMedia.as_index()], "featured_media");
        assert_eq!(columns[Sticky.as_index()], "sticky");
        assert_eq!(columns[Parent.as_index()], "parent");
        assert_eq!(columns[MenuOrder.as_index()], "menu_order");
        assert_eq!(columns[CommentStatus.as_index()], "comment_status");
        assert_eq!(columns[PingStatus.as_index()], "ping_status");
        assert_eq!(columns[Format.as_index()], "format");
        assert_eq!(columns[Meta.as_index()], "meta");
        assert_eq!(columns[GuidRendered.as_index()], "guid_rendered");
        assert_eq!(columns[TitleRendered.as_index()], "title_rendered");
        assert_eq!(columns[ContentRendered.as_index()], "content_rendered");
        assert_eq!(columns[ContentProtected.as_index()], "content_protected");
        assert_eq!(columns[ExcerptRaw.as_index()], "excerpt_raw");
        assert_eq!(columns[ExcerptRendered.as_index()], "excerpt_rendered");
        assert_eq!(columns[ExcerptProtected.as_index()], "excerpt_protected");
        assert_eq!(columns[LastFetchedAt.as_index()], "last_fetched_at");
        assert_eq!(columns[AdditionalFields.as_index()], "additional_fields");

        assert_eq!(columns.len(), AdditionalFields.as_index() + 1);
    }

    /// Verify that PostEmbedContextColumn enum values match the actual database schema.
    /// This test protects against column reordering in migrations breaking the positional index mapping.
    #[rstest]
    fn test_post_embed_context_column_enum_matches_schema(test_ctx: TestContext) {
        use PostEmbedContextColumn::*;

        let columns = get_table_column_names(&test_ctx.conn, "posts_embed_context");

        assert_eq!(columns[Rowid.as_index()], "rowid");
        assert_eq!(columns[DbSiteId.as_index()], "db_site_id");
        assert_eq!(columns[Id.as_index()], "id");
        assert_eq!(columns[Date.as_index()], "date");
        assert_eq!(columns[Link.as_index()], "link");
        assert_eq!(columns[Slug.as_index()], "slug");
        assert_eq!(columns[PostType.as_index()], "post_type");
        assert_eq!(columns[TitleRendered.as_index()], "title_rendered");
        assert_eq!(columns[Author.as_index()], "author");
        assert_eq!(columns[ExcerptRaw.as_index()], "excerpt_raw");
        assert_eq!(columns[ExcerptRendered.as_index()], "excerpt_rendered");
        assert_eq!(columns[ExcerptProtected.as_index()], "excerpt_protected");
        assert_eq!(columns[FeaturedMedia.as_index()], "featured_media");
        assert_eq!(columns[LastFetchedAt.as_index()], "last_fetched_at");
        assert_eq!(columns[AdditionalFields.as_index()], "additional_fields");

        assert_eq!(columns.len(), AdditionalFields.as_index() + 1);
    }

    #[rstest]
    #[case(PostBuilder::minimal().build())]
    #[case(PostBuilder::full().build())]
    #[case(PostBuilder::custom().build())]
    fn test_round_trip(mut test_ctx: TestContext, #[case] original_post: AnyPostWithEditContext) {
        // Insert into database using repository
        let entity_id = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &original_post)
            .expect("Failed to insert post");

        // Read back from database using PostRepository's select_by_entity_id
        let retrieved = test_ctx
            .post_repo
            .select_by_entity_id(&test_ctx.conn, &entity_id)
            .expect("Failed to read post")
            .expect("Post should exist");

        // Verify round-trip
        assert_eq!(retrieved.data.row_id, entity_id.rowid);
        assert_eq!(retrieved.data.db_site_id, test_ctx.site.row_id);
        assert_recent_timestamp(&retrieved.data.last_fetched_at);
        assert_eq!(retrieved.data.post, original_post);
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

        let entity_id = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .expect("Failed to upsert post");
        let retrieved = test_ctx
            .post_repo
            .select_by_entity_id(&test_ctx.conn, &entity_id)
            .expect("Failed to select post by entity_id")
            .expect("Post should exist");

        assert_eq!(retrieved.data.post.status, post_status);
    }

    #[rstest]
    fn test_round_trip_with_empty_json_arrays(mut test_ctx: TestContext) {
        let post = PostBuilder::minimal()
            .with_categories(vec![])
            .with_tags(vec![])
            .build();

        let entity_id = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .expect("Failed to upsert post");
        let retrieved = test_ctx
            .post_repo
            .select_by_entity_id(&test_ctx.conn, &entity_id)
            .expect("Failed to select post by entity_id")
            .expect("Post should exist");

        assert_eq!(retrieved.data.post.categories, None);
        assert_eq!(retrieved.data.post.tags, None);
    }

    #[rstest]
    fn test_repository_insert_and_select_by_entity_id(mut test_ctx: TestContext) {
        let post = PostBuilder::minimal().build();

        // Insert using repository
        let entity_id = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .expect("Failed to insert");

        // Select by entity_id
        let retrieved = test_ctx
            .post_repo
            .select_by_entity_id(&test_ctx.conn, &entity_id)
            .expect("Failed to select")
            .expect("Post should exist");

        assert_eq!(retrieved.data.row_id, entity_id.rowid);
        assert_eq!(retrieved.data.db_site_id, test_ctx.site.row_id);
        assert_eq!(retrieved.data.post, post);
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

        assert_eq!(retrieved.data.post.id, PostId(42));
        assert_eq!(retrieved.data.db_site_id, test_ctx.site.row_id);
        assert_eq!(retrieved.data.post, post);
    }

    #[rstest]
    fn test_repository_select_by_post_id_not_found(test_ctx: TestContext) {
        // Try to select non-existent post
        let result =
            test_ctx
                .post_repo
                .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(999));

        assert!(
            result.unwrap().is_none(),
            "Should return None when post doesn't exist"
        );
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
    fn test_repository_select_by_filter(mut test_ctx: TestContext) {
        // Insert posts with different statuses
        let published_post = PostBuilder::minimal()
            .with_status(wp_api::posts::PostStatus::Publish)
            .build();
        let draft_post = PostBuilder::minimal()
            .with_status(wp_api::posts::PostStatus::Draft)
            .build();

        test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &published_post)
            .unwrap();
        test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &draft_post)
            .unwrap();

        // Filter by publish status
        let published = test_ctx
            .post_repo
            .select_by_filter(
                &test_ctx.conn,
                &test_ctx.site,
                Some(&wp_api::posts::PostStatus::Publish),
            )
            .unwrap();
        assert_eq!(published.len(), 1);
        assert_eq!(
            published[0].data.post.status,
            wp_api::posts::PostStatus::Publish
        );

        // Filter by draft status
        let drafts = test_ctx
            .post_repo
            .select_by_filter(
                &test_ctx.conn,
                &test_ctx.site,
                Some(&wp_api::posts::PostStatus::Draft),
            )
            .unwrap();
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].data.post.status, wp_api::posts::PostStatus::Draft);

        // No filter - returns all
        let all = test_ctx
            .post_repo
            .select_by_filter(&test_ctx.conn, &test_ctx.site, None)
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
        let entity_ids = test_ctx
            .post_repo
            .upsert_batch(&mut test_ctx.conn, &test_ctx.site, &posts)
            .unwrap();
        assert_eq!(entity_ids.len(), 3);

        // Verify all were inserted
        assert_eq!(
            test_ctx
                .post_repo
                .count(&test_ctx.conn, &test_ctx.site)
                .unwrap(),
            3
        );

        // Verify can retrieve each
        entity_ids.iter().for_each(|entity_id| {
            test_ctx
                .post_repo
                .select_by_entity_id(&test_ctx.conn, entity_id)
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
        let result = test_ctx
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
    fn test_repository_delete_by_entity_id(mut test_ctx: TestContext) {
        let post = PostBuilder::minimal().with_id(42).build();
        let entity_id = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();

        // Verify exists
        test_ctx
            .post_repo
            .select_by_entity_id(&test_ctx.conn, &entity_id)
            .expect("Should not error")
            .expect("Post should exist");

        // Delete
        let deleted = test_ctx
            .post_repo
            .delete_by_entity_id(&test_ctx.conn, &entity_id)
            .unwrap();
        assert_eq!(deleted, 1);

        // Verify no longer exists
        let result = test_ctx
            .post_repo
            .select_by_entity_id(&test_ctx.conn, &entity_id)
            .unwrap();
        assert!(result.is_none(), "Post should not exist after deletion");
    }

    #[rstest]
    fn test_delete_by_entity_id_deletes_terms(mut test_ctx: TestContext) {
        // Insert post with terms
        let post = PostBuilder::minimal()
            .with_id(500)
            .with_categories(vec![wp_api::terms::TermId(1), wp_api::terms::TermId(2)])
            .build();
        let entity_id = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();

        // Verify terms exist
        let terms = test_ctx
            .term_repo
            .get_all_terms_for_object(&test_ctx.conn, &test_ctx.site, post.id.0)
            .unwrap();
        assert!(!terms.is_empty());

        // Delete post by entity_id
        test_ctx
            .post_repo
            .delete_by_entity_id(&test_ctx.conn, &entity_id)
            .unwrap();

        // Verify terms were also deleted
        let terms_after = test_ctx
            .term_repo
            .get_all_terms_for_object(&test_ctx.conn, &test_ctx.site, post.id.0)
            .unwrap();
        assert!(terms_after.is_empty());
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
        let entity_id = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();

        // Verify it was inserted
        let retrieved = test_ctx
            .post_repo
            .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(100))
            .expect("Failed to select post by post_id")
            .expect("Post should exist after insert");
        assert_eq!(retrieved.data.row_id, entity_id.rowid);
        assert_eq!(retrieved.data.db_site_id, test_ctx.site.row_id);
        assert_eq!(retrieved.data.post.status, PostStatus::Draft);
    }

    #[rstest]
    fn test_repository_upsert_updates_existing_post(mut test_ctx: TestContext) {
        // Insert initial post
        let post = PostBuilder::minimal()
            .with_id(200)
            .with_status(PostStatus::Draft)
            .with_slug("original-slug")
            .build();

        let original_entity_id = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();

        // Upsert with updated data
        let updated_post = PostBuilder::minimal()
            .with_id(200)
            .with_status(PostStatus::Publish)
            .with_slug("updated-slug")
            .build();

        let new_entity_id = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &updated_post)
            .unwrap();

        // EntityId should be the same (it's an update, not delete+insert)
        assert_eq!(original_entity_id, new_entity_id);

        // Verify the update
        let retrieved = test_ctx
            .post_repo
            .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(200))
            .expect("Failed to select post by post_id")
            .expect("Post should exist after update");
        assert_eq!(retrieved.data.post.status, PostStatus::Publish);
        assert_eq!(retrieved.data.post.slug, "updated-slug");

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
        let entity_id = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();

        // Verify post was inserted
        let retrieved = test_ctx
            .post_repo
            .select_by_entity_id(&test_ctx.conn, &entity_id)
            .expect("Failed to select post by entity_id")
            .expect("Post should exist");
        assert_eq!(retrieved.data.post.id, PostId(300));

        // Verify categories were inserted
        assert_eq!(retrieved.data.post.categories.as_ref().unwrap().len(), 2);
        assert!(
            retrieved
                .data
                .post
                .categories
                .as_ref()
                .unwrap()
                .contains(&wp_api::terms::TermId(1))
        );
        assert!(
            retrieved
                .data
                .post
                .categories
                .as_ref()
                .unwrap()
                .contains(&wp_api::terms::TermId(2))
        );

        // Verify tags were inserted
        assert_eq!(retrieved.data.post.tags.as_ref().unwrap().len(), 2);
        assert!(
            retrieved
                .data
                .post
                .tags
                .as_ref()
                .unwrap()
                .contains(&wp_api::terms::TermId(10))
        );
        assert!(
            retrieved
                .data
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
            .expect("Failed to select post by post_id")
            .expect("Post should exist");

        // Categories: should have 1, 3 (not 2)
        assert_eq!(retrieved.data.post.categories.as_ref().unwrap().len(), 2);
        assert!(
            retrieved
                .data
                .post
                .categories
                .as_ref()
                .unwrap()
                .contains(&wp_api::terms::TermId(1))
        );
        assert!(
            retrieved
                .data
                .post
                .categories
                .as_ref()
                .unwrap()
                .contains(&wp_api::terms::TermId(3))
        );
        assert!(
            !retrieved
                .data
                .post
                .categories
                .as_ref()
                .unwrap()
                .contains(&wp_api::terms::TermId(2))
        );

        // Tags: should only have 10 (not 20, 30)
        assert_eq!(retrieved.data.post.tags.as_ref().unwrap().len(), 1);
        assert_eq!(
            retrieved.data.post.tags.as_ref().unwrap()[0],
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
    fn test_select_by_entity_id_populates_terms(mut test_ctx: TestContext) {
        // Insert post with terms
        let post = PostBuilder::minimal()
            .with_id(600)
            .with_categories(vec![wp_api::terms::TermId(5)])
            .build();

        let entity_id = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();

        // Select by entity_id should populate terms
        let retrieved = test_ctx
            .post_repo
            .select_by_entity_id(&test_ctx.conn, &entity_id)
            .expect("Failed to select post by entity_id")
            .expect("Post should exist");
        assert_eq!(
            retrieved.data.post.categories,
            Some(vec![wp_api::terms::TermId(5)])
        );
    }

    #[rstest]
    fn test_insert_sets_last_fetched_at(mut test_ctx: TestContext) {
        let post = PostBuilder::minimal().build();

        // Insert post
        let entity_id = test_ctx
            .post_repo
            .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
            .unwrap();

        // Retrieve and validate last_fetched_at
        let retrieved = test_ctx
            .post_repo
            .select_by_entity_id(&test_ctx.conn, &entity_id)
            .expect("Failed to select post by entity_id")
            .expect("Post should exist");

        // Validate timestamp is recent and valid
        assert_recent_timestamp(&retrieved.data.last_fetched_at);
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
            .expect("Failed to select post by post_id")
            .expect("Post should exist")
            .data
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
            .expect("Failed to select post by post_id")
            .expect("Post should exist")
            .data
            .last_fetched_at;

        // last_fetched_at should be updated (different)
        assert_ne!(first_fetch, second_fetch);

        // Both should be valid timestamps
        assert!(first_fetch.ends_with('Z'));
        assert!(second_fetch.ends_with('Z'));
    }
}
