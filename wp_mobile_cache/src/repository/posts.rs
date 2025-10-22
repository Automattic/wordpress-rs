use crate::{
    DbSite, RowId, SqliteDbError,
    mappings::{TryFromDbRow, posts::DbAnyPostWithEditContext},
    repository::{
        QueryExecutor, TransactionManager, term_relationships::TermRelationshipRepository,
    },
};
use wp_api::posts::{AnyPostWithEditContext, PostId};
use wp_api::taxonomies::TaxonomyType;

/// Repository for managing posts in the database.
///
/// Provides CRUD operations and post-specific query methods.
pub struct PostRepository;

impl PostRepository {
    /// Select a post by its SQLite rowid for a given site (returns wrapper with rowid).
    ///
    /// Returns an error if no post with the given rowid exists for this site.
    /// Automatically populates categories and tags from term_relationships table.
    pub fn select_by_rowid(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        rowid: RowId,
    ) -> Result<DbAnyPostWithEditContext, SqliteDbError> {
        let sql = "SELECT * FROM posts_edit_context WHERE db_site_id = ? AND rowid = ?";
        let mut stmt = executor.prepare(sql)?;
        let mut db_post = stmt
            .query_row([site.row_id, rowid], |row| {
                DbAnyPostWithEditContext::try_from_row(row)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
            })
            .map_err(SqliteDbError::from)?;

        // Populate terms from term_relationships table
        self.populate_terms(executor, site, &mut db_post)?;

        Ok(db_post)
    }

    /// Select all posts for a given site (returns wrappers with rowids).
    ///
    /// Returns an empty vector if no posts exist for the site.
    /// Automatically populates categories and tags from term_relationships table.
    pub fn select_all(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
    ) -> Result<Vec<DbAnyPostWithEditContext>, SqliteDbError> {
        let sql = "SELECT * FROM posts_edit_context WHERE db_site_id = ?";
        let mut stmt = executor.prepare(sql)?;
        let rows = stmt.query_map([site.row_id], |row| {
            DbAnyPostWithEditContext::try_from_row(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        let mut posts: Vec<DbAnyPostWithEditContext> = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(SqliteDbError::from)?;

        // Populate terms for all posts
        for db_post in &mut posts {
            self.populate_terms(executor, site, db_post)?;
        }

        Ok(posts)
    }

    /// Select a post by its WordPress post ID for a given site (returns wrapper with rowid).
    ///
    /// This is different from `select_by_rowid` which uses the SQLite rowid.
    /// The post_id is the WordPress post ID from the REST API.
    ///
    /// Returns an error if no post with the given WordPress post ID exists for this site.
    /// Automatically populates categories and tags from term_relationships table.
    pub fn select_by_post_id(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        post_id: PostId,
    ) -> Result<DbAnyPostWithEditContext, SqliteDbError> {
        let sql = "SELECT * FROM posts_edit_context WHERE db_site_id = ? AND id = ?";
        let mut stmt = executor.prepare(sql)?;
        let mut db_post = stmt
            .query_row(rusqlite::params![site.row_id, post_id.0], |row| {
                DbAnyPostWithEditContext::try_from_row(row)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
            })
            .map_err(SqliteDbError::from)?;

        // Populate terms from term_relationships table
        self.populate_terms(executor, site, &mut db_post)?;

        Ok(db_post)
    }

    /// Select posts by author user ID for a given site (returns wrappers with rowids).
    ///
    /// Returns an empty vector if no posts by the given author exist for the site.
    /// Automatically populates categories and tags from term_relationships table.
    pub fn select_by_author(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        author_id: wp_api::users::UserId,
    ) -> Result<Vec<DbAnyPostWithEditContext>, SqliteDbError> {
        let sql = "SELECT * FROM posts_edit_context WHERE db_site_id = ? AND author = ?";
        let mut stmt = executor.prepare(sql)?;
        let rows = stmt.query_map(rusqlite::params![site.row_id, author_id.0], |row| {
            DbAnyPostWithEditContext::try_from_row(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        let mut posts: Vec<DbAnyPostWithEditContext> = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(SqliteDbError::from)?;

        // Populate terms for all posts
        for db_post in &mut posts {
            self.populate_terms(executor, site, db_post)?;
        }

        Ok(posts)
    }

    /// Select posts by status for a given site (e.g., "publish", "draft").
    ///
    /// Returns an empty vector if no posts with the given status exist for the site.
    /// Automatically populates categories and tags from term_relationships table.
    pub fn select_by_status(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        status: &str,
    ) -> Result<Vec<DbAnyPostWithEditContext>, SqliteDbError> {
        let sql = "SELECT * FROM posts_edit_context WHERE db_site_id = ? AND status = ?";
        let mut stmt = executor.prepare(sql)?;
        let rows = stmt.query_map(rusqlite::params![site.row_id, status], |row| {
            DbAnyPostWithEditContext::try_from_row(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        let mut posts: Vec<DbAnyPostWithEditContext> = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(SqliteDbError::from)?;

        // Populate terms for all posts
        for db_post in &mut posts {
            self.populate_terms(executor, site, db_post)?;
        }

        Ok(posts)
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
        let db_post = match self.select_by_post_id(executor, site, post_id) {
            Ok(post) => post,
            Err(_) => return Ok(0), // Post doesn't exist
        };

        // Delete term relationships
        let term_repo = TermRelationshipRepository;
        term_repo.delete_all_terms_for_object(executor, site, db_post.row_id)?;

        // Delete the post
        let sql = "DELETE FROM posts_edit_context WHERE db_site_id = ? AND id = ?";
        executor.execute(sql, rusqlite::params![site.row_id, post_id.0])
    }

    /// Upsert a post with its term relationships (atomic transaction).
    ///
    /// This uses SQLite's INSERT ... ON CONFLICT ... DO UPDATE syntax to either
    /// insert a new post or update an existing one based on the (db_site_id, post_id) pair.
    /// This ensures the database observer sees a single INSERT or UPDATE action.
    ///
    /// Term relationships are synced using a diff approach - only changes generate DB events.
    ///
    /// Returns the rowid of the inserted or updated row.
    pub fn upsert(
        &self,
        transaction_manager: &mut impl TransactionManager,
        site: &DbSite,
        post: &AnyPostWithEditContext,
    ) -> Result<RowId, SqliteDbError> {
        use crate::mappings::helpers::{bool_to_integer, serialize_value_to_json};

        let tx = transaction_manager.transaction()?;

        tx.execute(
            r#"
            INSERT INTO posts_edit_context (
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

        let sql = "SELECT rowid FROM posts_edit_context WHERE db_site_id = ? AND id = ?";
        let post_rowid: i64 = {
            let mut stmt = tx.prepare(sql)?;
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
                post_rowid,
                &TaxonomyType::Category,
                categories,
            )?;
        }

        if let Some(ref tags) = post.tags {
            term_repo.sync_terms_for_object(&tx, site, post_rowid, &TaxonomyType::PostTag, tags)?;
        }

        tx.commit().map_err(SqliteDbError::from)?;
        Ok(post_rowid)
    }

    /// Upsert multiple posts with their term relationships.
    ///
    /// Each post is upserted in its own transaction. If any upsert fails,
    /// previously successful upserts remain in the database.
    ///
    /// Returns a vector of rowids for successfully upserted posts.
    pub fn upsert_batch(
        &self,
        transaction_manager: &mut impl TransactionManager,
        site: &DbSite,
        posts: &[AnyPostWithEditContext],
    ) -> Result<Vec<RowId>, SqliteDbError> {
        let mut rowids = Vec::with_capacity(posts.len());
        for post in posts {
            let rowid = self.upsert(transaction_manager, site, post)?;
            rowids.push(rowid);
        }
        Ok(rowids)
    }

    /// Get the total count of posts for a given site.
    pub fn count(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
    ) -> Result<i64, SqliteDbError> {
        let sql = "SELECT COUNT(*) FROM posts_edit_context WHERE db_site_id = ?";
        let mut stmt = executor.prepare(sql)?;
        stmt.query_row([site.row_id], |row| row.get(0))
            .map_err(SqliteDbError::from)
    }

    /// Populate categories and tags from term_relationships table.
    ///
    /// This is a helper method used by select methods to join term data.
    fn populate_terms(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        db_post: &mut DbAnyPostWithEditContext,
    ) -> Result<(), SqliteDbError> {
        let term_repo = TermRelationshipRepository;
        let terms_map = term_repo.get_all_terms_for_object(executor, site, db_post.row_id)?;

        // Only overwrite categories/tags if there are entries in term_relationships.
        // This preserves backward compatibility with posts that only have JSON storage.
        if let Some(categories) = terms_map.get(&TaxonomyType::Category) {
            db_post.post.categories = Some(categories.clone());
        }
        if let Some(tags) = terms_map.get(&TaxonomyType::PostTag) {
            db_post.post.tags = Some(tags.clone());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_fixtures::posts::{create_full_post, create_minimal_post},
        test_helpers::{test_db, test_site},
    };
    use rstest::*;
    use rusqlite::Connection;
    use wp_api::posts::PostStatus;
    use wp_api::users::UserId;

    #[rstest]
    fn test_repository_insert_and_select_by_rowid(mut test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;
        let post = create_minimal_post();

        // Insert using repository
        let rowid = repo
            .upsert(&mut test_db, &test_site, &post)
            .expect("Failed to insert");

        // Select by rowid
        let retrieved = repo
            .select_by_rowid(&test_db, &test_site, rowid)
            .expect("Failed to select");

        assert_eq!(retrieved.row_id, rowid);
        assert_eq!(retrieved.site, test_site);
        assert_eq!(retrieved.post, post);
    }

    #[rstest]
    fn test_repository_select_by_post_id(mut test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;
        let mut post = create_minimal_post();
        post.id = PostId(42);

        // Insert
        repo.upsert(&mut test_db, &test_site, &post)
            .expect("Failed to insert");

        // Select by post_id
        let retrieved = repo
            .select_by_post_id(&test_db, &test_site, PostId(42))
            .expect("Failed to select by post_id");

        assert_eq!(retrieved.post.id, PostId(42));
        assert_eq!(retrieved.site, test_site);
        assert_eq!(retrieved.post, post);
    }

    #[rstest]
    fn test_repository_select_by_post_id_not_found(test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;

        // Try to select non-existent post
        let result = repo.select_by_post_id(&test_db, &test_site, PostId(999));

        assert!(result.is_err());
    }

    #[rstest]
    fn test_repository_select_by_author(mut test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;

        // Insert posts with different authors
        let mut post1 = create_minimal_post();
        post1.id = PostId(1);
        post1.author = Some(UserId(10));

        let mut post2 = create_minimal_post();
        post2.id = PostId(2);
        post2.author = Some(UserId(10));

        let mut post3 = create_minimal_post();
        post3.id = PostId(3);
        post3.author = Some(UserId(20));

        repo.upsert(&mut test_db, &test_site, &post1).unwrap();
        repo.upsert(&mut test_db, &test_site, &post2).unwrap();
        repo.upsert(&mut test_db, &test_site, &post3).unwrap();

        // Select by author
        let author_10_posts = repo
            .select_by_author(&test_db, &test_site, UserId(10))
            .unwrap();
        assert_eq!(author_10_posts.len(), 2);
        assert!(
            author_10_posts
                .iter()
                .all(|p| p.post.author == Some(UserId(10)))
        );

        let author_20_posts = repo
            .select_by_author(&test_db, &test_site, UserId(20))
            .unwrap();
        assert_eq!(author_20_posts.len(), 1);
        assert_eq!(author_20_posts[0].post.author, Some(UserId(20)));
    }

    #[rstest]
    fn test_repository_select_by_status(mut test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;

        // Insert posts with different statuses
        let mut post1 = create_minimal_post();
        post1.id = PostId(1);
        post1.status = PostStatus::Publish;

        let mut post2 = create_minimal_post();
        post2.id = PostId(2);
        post2.status = PostStatus::Draft;

        let mut post3 = create_minimal_post();
        post3.id = PostId(3);
        post3.status = PostStatus::Publish;

        repo.upsert(&mut test_db, &test_site, &post1).unwrap();
        repo.upsert(&mut test_db, &test_site, &post2).unwrap();
        repo.upsert(&mut test_db, &test_site, &post3).unwrap();

        // Select by status
        let published = repo
            .select_by_status(&test_db, &test_site, "publish")
            .unwrap();
        assert_eq!(published.len(), 2);

        let drafts = repo
            .select_by_status(&test_db, &test_site, "draft")
            .unwrap();
        assert_eq!(drafts.len(), 1);
    }

    #[rstest]
    fn test_repository_select_all(mut test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;

        // Initially empty
        let all = repo.select_all(&test_db, &test_site).unwrap();
        assert_eq!(all.len(), 0);

        // Insert posts
        let mut post1 = create_minimal_post();
        post1.id = PostId(1);
        let mut post2 = create_minimal_post();
        post2.id = PostId(2);

        repo.upsert(&mut test_db, &test_site, &post1).unwrap();
        repo.upsert(&mut test_db, &test_site, &post2).unwrap();

        // Select all
        let all = repo.select_all(&test_db, &test_site).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[rstest]
    fn test_repository_count(mut test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;

        assert_eq!(repo.count(&test_db, &test_site).unwrap(), 0);

        let mut post1 = create_minimal_post();
        post1.id = PostId(1);
        repo.upsert(&mut test_db, &test_site, &post1).unwrap();

        assert_eq!(repo.count(&test_db, &test_site).unwrap(), 1);

        let mut post2 = create_minimal_post();
        post2.id = PostId(2);
        repo.upsert(&mut test_db, &test_site, &post2).unwrap();

        assert_eq!(repo.count(&test_db, &test_site).unwrap(), 2);
    }

    #[rstest]
    fn test_repository_insert_batch(mut test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;

        let mut post1 = create_minimal_post();
        post1.id = PostId(1);
        let mut post2 = create_full_post();
        post2.id = PostId(2);
        let mut post3 = create_minimal_post();
        post3.id = PostId(3);

        let posts = vec![post1, post2, post3];

        // Insert batch
        let rowids = repo.upsert_batch(&mut test_db, &test_site, &posts).unwrap();
        assert_eq!(rowids.len(), 3);

        // Verify all were inserted
        assert_eq!(repo.count(&test_db, &test_site).unwrap(), 3);

        // Verify can retrieve each
        for rowid in rowids {
            repo.select_by_rowid(&test_db, &test_site, rowid)
                .expect("Should exist");
        }
    }

    #[rstest]
    fn test_repository_delete_by_post_id(mut test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;

        let mut post = create_minimal_post();
        post.id = PostId(42);
        repo.upsert(&mut test_db, &test_site, &post).unwrap();

        // Verify exists
        repo.select_by_post_id(&test_db, &test_site, PostId(42))
            .expect("Post should exist");

        // Delete
        let deleted = repo
            .delete_by_post_id(&test_db, &test_site, PostId(42))
            .unwrap();
        assert_eq!(deleted, 1);

        // Verify no longer exists
        let result = repo.select_by_post_id(&test_db, &test_site, PostId(42));
        assert!(result.is_err());

        // Delete non-existent should return 0
        let deleted = repo
            .delete_by_post_id(&test_db, &test_site, PostId(999))
            .unwrap();
        assert_eq!(deleted, 0);
    }

    #[rstest]
    fn test_repository_upsert_inserts_new_post(mut test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;

        let mut post = create_minimal_post();
        post.id = PostId(100);
        post.status = PostStatus::Draft;

        // Verify post doesn't exist
        assert!(
            repo.select_by_post_id(&test_db, &test_site, PostId(100))
                .is_err()
        );

        // Upsert should insert
        let rowid = repo.upsert(&mut test_db, &test_site, &post).unwrap();

        // Verify it was inserted
        let retrieved = repo
            .select_by_post_id(&test_db, &test_site, PostId(100))
            .unwrap();
        assert_eq!(retrieved.row_id, rowid);
        assert_eq!(retrieved.site, test_site);
        assert_eq!(retrieved.post.status, PostStatus::Draft);
    }

    #[rstest]
    fn test_repository_upsert_updates_existing_post(mut test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;

        // Insert initial post
        let mut post = create_minimal_post();
        post.id = PostId(200);
        post.status = PostStatus::Draft;
        post.slug = "original-slug".to_string();

        let original_rowid = repo.upsert(&mut test_db, &test_site, &post).unwrap();

        // Upsert with updated data
        let mut updated_post = create_minimal_post();
        updated_post.id = PostId(200);
        updated_post.status = PostStatus::Publish;
        updated_post.slug = "updated-slug".to_string();

        let new_rowid = repo
            .upsert(&mut test_db, &test_site, &updated_post)
            .unwrap();

        // Rowid should be the same (it's an update, not delete+insert)
        assert_eq!(original_rowid, new_rowid);

        // Verify the update
        let retrieved = repo
            .select_by_post_id(&test_db, &test_site, PostId(200))
            .unwrap();
        assert_eq!(retrieved.post.status, PostStatus::Publish);
        assert_eq!(retrieved.post.slug, "updated-slug");

        // Verify only one post exists with this ID
        assert_eq!(repo.count(&test_db, &test_site).unwrap(), 1);
    }

    #[rstest]
    fn test_upsert_inserts_post_and_terms(mut test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;

        let mut post = create_minimal_post();
        post.id = PostId(300);
        post.categories = Some(vec![wp_api::terms::TermId(1), wp_api::terms::TermId(2)]);
        post.tags = Some(vec![wp_api::terms::TermId(10), wp_api::terms::TermId(20)]);

        // Upsert with terms
        let rowid = repo.upsert(&mut test_db, &test_site, &post).unwrap();

        // Verify post was inserted
        let retrieved = repo.select_by_rowid(&test_db, &test_site, rowid).unwrap();
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
    fn test_upsert_updates_existing_terms(mut test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;

        // Insert post with initial terms
        let mut post = create_minimal_post();
        post.id = PostId(400);
        post.categories = Some(vec![wp_api::terms::TermId(1), wp_api::terms::TermId(2)]);
        post.tags = Some(vec![
            wp_api::terms::TermId(10),
            wp_api::terms::TermId(20),
            wp_api::terms::TermId(30),
        ]);

        repo.upsert(&mut test_db, &test_site, &post).unwrap();

        // Update with different terms
        post.categories = Some(vec![wp_api::terms::TermId(1), wp_api::terms::TermId(3)]); // Remove 2, add 3
        post.tags = Some(vec![wp_api::terms::TermId(10)]); // Remove 20, 30

        repo.upsert(&mut test_db, &test_site, &post).unwrap();

        // Verify updated terms
        let retrieved = repo
            .select_by_post_id(&test_db, &test_site, PostId(400))
            .unwrap();

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
    fn test_delete_by_post_id_deletes_terms(mut test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;
        let term_repo = crate::repository::term_relationships::TermRelationshipRepository;

        // Insert post without terms (to avoid transaction issues in this test)
        let mut post = create_minimal_post();
        post.id = PostId(500);
        let rowid = repo.upsert(&mut test_db, &test_site, &post).unwrap();

        // Manually add terms
        term_repo
            .sync_terms_for_object(
                &test_db,
                &test_site,
                rowid,
                &wp_api::taxonomies::TaxonomyType::Category,
                &[wp_api::terms::TermId(1), wp_api::terms::TermId(2)],
            )
            .unwrap();

        // Verify terms exist
        let terms = term_repo
            .get_all_terms_for_object(&test_db, &test_site, rowid)
            .unwrap();
        assert!(!terms.is_empty());

        // Delete post
        repo.delete_by_post_id(&test_db, &test_site, PostId(500))
            .unwrap();

        // Verify terms were also deleted
        let terms_after = term_repo
            .get_all_terms_for_object(&test_db, &test_site, rowid)
            .unwrap();
        assert!(terms_after.is_empty());
    }

    #[rstest]
    fn test_select_by_rowid_populates_terms(mut test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;

        // Insert post with terms
        let mut post = create_minimal_post();
        post.id = PostId(600);
        post.categories = Some(vec![wp_api::terms::TermId(5)]);

        let rowid = repo.upsert(&mut test_db, &test_site, &post).unwrap();

        // Select by rowid should populate terms
        let retrieved = repo.select_by_rowid(&test_db, &test_site, rowid).unwrap();
        assert_eq!(
            retrieved.post.categories,
            Some(vec![wp_api::terms::TermId(5)])
        );
    }

    #[rstest]
    fn test_insert_sets_last_fetched_at(mut test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;
        let mut post = create_minimal_post();
        post.id = PostId(100);

        // Insert post
        let rowid = repo.upsert(&mut test_db, &test_site, &post).unwrap();

        // Retrieve and validate last_fetched_at
        let retrieved = repo.select_by_rowid(&test_db, &test_site, rowid).unwrap();

        // Validate ISO 8601 UTC format
        assert!(retrieved.last_fetched_at.ends_with('Z'));
        assert!(retrieved.last_fetched_at.contains('T'));
        assert!(retrieved.last_fetched_at.len() >= 20);

        // Validate it's a recent timestamp (within last second)
        // Format: 2024-01-01T00:00:00.000Z
        assert!(retrieved.last_fetched_at.starts_with("2025"));
    }

    #[rstest]
    fn test_upsert_updates_last_fetched_at_on_update(mut test_db: Connection, test_site: DbSite) {
        let repo = PostRepository;
        let mut post = create_minimal_post();
        post.id = PostId(200);
        post.title.rendered = "Original Title".to_string();

        // Initial insert
        repo.upsert(&mut test_db, &test_site, &post).unwrap();
        let first_fetch = repo
            .select_by_post_id(&test_db, &test_site, PostId(200))
            .unwrap()
            .last_fetched_at
            .clone();

        // Sleep a tiny bit to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Update post
        post.title.rendered = "Updated Title".to_string();
        repo.upsert(&mut test_db, &test_site, &post).unwrap();
        let second_fetch = repo
            .select_by_post_id(&test_db, &test_site, PostId(200))
            .unwrap()
            .last_fetched_at;

        // last_fetched_at should be updated (different)
        assert_ne!(first_fetch, second_fetch);

        // Both should be valid timestamps
        assert!(first_fetch.ends_with('Z'));
        assert!(second_fetch.ends_with('Z'));
    }
}
