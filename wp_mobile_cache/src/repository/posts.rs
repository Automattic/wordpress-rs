use crate::{
    SiteId, SqliteDbError,
    mappings::{TryFromDbRow, posts::DbAnyPostWithEditContext},
    repository::{QueryExecutor, Repository},
};
use wp_api::posts::{AnyPostWithEditContext, PostId};

/// Repository for managing posts in the database.
///
/// Provides both common CRUD operations (via Repository trait) and
/// post-specific query methods.
pub struct PostRepository;

impl Repository for PostRepository {
    type Entity = AnyPostWithEditContext;
}

impl PostRepository {
    /// Select a post by its SQLite rowid and site_id (returns wrapper with rowid).
    ///
    /// Returns an error if no post with the given rowid and site_id exists.
    pub fn select_by_rowid(
        &self,
        executor: &impl QueryExecutor,
        site_id: SiteId,
        rowid: i64,
    ) -> Result<DbAnyPostWithEditContext, SqliteDbError> {
        let sql = "SELECT * FROM posts_edit_context WHERE site_id = ? AND rowid = ?";
        let mut stmt = executor.prepare(sql)?;
        stmt.query_row([site_id.0, rowid], |row| {
            DbAnyPostWithEditContext::try_from_row(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })
        .map_err(SqliteDbError::from)
    }

    /// Select all posts for a given site (returns wrappers with rowids).
    ///
    /// Returns an empty vector if no posts exist for the site.
    pub fn select_all(
        &self,
        executor: &impl QueryExecutor,
        site_id: SiteId,
    ) -> Result<Vec<DbAnyPostWithEditContext>, SqliteDbError> {
        let sql = "SELECT * FROM posts_edit_context WHERE site_id = ?";
        let mut stmt = executor.prepare(sql)?;
        let rows = stmt.query_map([site_id.0], |row| {
            DbAnyPostWithEditContext::try_from_row(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(SqliteDbError::from)
    }

    /// Select a post by its WordPress post ID and site_id (returns wrapper with rowid).
    ///
    /// This is different from `select_by_rowid` which uses the SQLite rowid.
    /// The post_id is the WordPress post ID from the REST API.
    ///
    /// Returns an error if no post with the given ID and site_id exists.
    pub fn select_by_post_id(
        &self,
        executor: &impl QueryExecutor,
        site_id: SiteId,
        post_id: PostId,
    ) -> Result<DbAnyPostWithEditContext, SqliteDbError> {
        let sql = "SELECT * FROM posts_edit_context WHERE site_id = ? AND id = ?";
        let mut stmt = executor.prepare(sql)?;
        stmt.query_row([site_id.0, post_id.0], |row| {
            DbAnyPostWithEditContext::try_from_row(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })
        .map_err(SqliteDbError::from)
    }

    /// Select posts by author user ID for a given site (returns wrappers with rowids).
    ///
    /// Returns an empty vector if no posts by the given author exist for the site.
    pub fn select_by_author(
        &self,
        executor: &impl QueryExecutor,
        site_id: SiteId,
        author_id: wp_api::users::UserId,
    ) -> Result<Vec<DbAnyPostWithEditContext>, SqliteDbError> {
        let sql = "SELECT * FROM posts_edit_context WHERE site_id = ? AND author = ?";
        let mut stmt = executor.prepare(sql)?;
        let rows = stmt.query_map([site_id.0, author_id.0], |row| {
            DbAnyPostWithEditContext::try_from_row(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(SqliteDbError::from)
    }

    /// Select posts by status for a given site (e.g., "publish", "draft").
    ///
    /// Returns an empty vector if no posts with the given status exist for the site.
    pub fn select_by_status(
        &self,
        executor: &impl QueryExecutor,
        site_id: SiteId,
        status: &str,
    ) -> Result<Vec<DbAnyPostWithEditContext>, SqliteDbError> {
        let sql = "SELECT * FROM posts_edit_context WHERE site_id = ? AND status = ?";
        let mut stmt = executor.prepare(sql)?;
        let rows = stmt.query_map([site_id.0.to_string(), status.to_string()], |row| {
            DbAnyPostWithEditContext::try_from_row(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(SqliteDbError::from)
    }

    /// Delete a post by its WordPress post ID and site_id.
    ///
    /// Returns the number of rows deleted (0 or 1).
    pub fn delete_by_post_id(
        &self,
        executor: &impl QueryExecutor,
        site_id: SiteId,
        post_id: PostId,
    ) -> Result<usize, SqliteDbError> {
        let sql = "DELETE FROM posts_edit_context WHERE site_id = ? AND id = ?";
        executor.execute(sql, [site_id.0, post_id.0])
    }

    /// Upsert a post (insert or update) by its WordPress post ID and site_id.
    ///
    /// This uses SQLite's INSERT ... ON CONFLICT ... DO UPDATE syntax to either
    /// insert a new post or update an existing one based on the (site_id, post_id) pair.
    /// This ensures the database observer sees a single INSERT or UPDATE action,
    /// not a DELETE followed by INSERT.
    ///
    /// Returns the rowid of the inserted or updated row.
    pub fn upsert(
        &self,
        conn: &rusqlite::Connection,
        site_id: SiteId,
        post: &AnyPostWithEditContext,
    ) -> Result<i64, SqliteDbError> {
        use crate::mappings::helpers::{
            bool_to_integer, serialize_json_id_array, serialize_value_to_json,
        };

        conn.execute(
            r#"
            INSERT INTO posts_edit_context (
                site_id, id, date, date_gmt, link, modified, modified_gmt, slug, status, post_type,
                password, template, permalink_template, generated_slug, author, featured_media,
                sticky, parent, menu_order, comment_status, ping_status, format, meta,
                categories, tags, guid_raw, guid_rendered, title_raw, title_rendered,
                content_raw, content_rendered, content_protected, content_block_version,
                excerpt_raw, excerpt_rendered, excerpt_protected
            ) VALUES (
                :site_id, :id, :date, :date_gmt, :link, :modified, :modified_gmt, :slug, :status, :post_type,
                :password, :template, :permalink_template, :generated_slug, :author, :featured_media,
                :sticky, :parent, :menu_order, :comment_status, :ping_status, :format, :meta,
                :categories, :tags, :guid_raw, :guid_rendered, :title_raw, :title_rendered,
                :content_raw, :content_rendered, :content_protected, :content_block_version,
                :excerpt_raw, :excerpt_rendered, :excerpt_protected
            )
            ON CONFLICT(site_id, id) DO UPDATE SET
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
                categories = excluded.categories,
                tags = excluded.tags,
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
                excerpt_protected = excluded.excerpt_protected
            "#,
            rusqlite::named_params! {
                ":site_id": site_id.0,
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
                ":categories": serialize_json_id_array(&post.categories, |t| t.0)?,
                ":tags": serialize_json_id_array(&post.tags, |t| t.0)?,
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

        Ok(conn.last_insert_rowid())
    }

    /// Get the total count of posts for a given site.
    pub fn count(
        &self,
        executor: &impl QueryExecutor,
        site_id: SiteId,
    ) -> Result<i64, SqliteDbError> {
        let sql = "SELECT COUNT(*) FROM posts_edit_context WHERE site_id = ?";
        let mut stmt = executor.prepare(sql)?;
        stmt.query_row([site_id.0], |row| row.get(0))
            .map_err(SqliteDbError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_fixtures::posts::{create_full_post, create_minimal_post},
        unit_test_common::setup_test_db,
    };
    use wp_api::posts::PostStatus;
    use wp_api::users::UserId;

    const TEST_SITE_ID: SiteId = SiteId(1);

    #[test]
    fn test_repository_insert_and_select_by_rowid() {
        let conn = setup_test_db();
        let repo = PostRepository;
        let post = create_minimal_post();

        // Insert using repository
        let rowid = repo
            .insert(&conn, &post, TEST_SITE_ID)
            .expect("Failed to insert");

        // Select by rowid
        let retrieved = repo
            .select_by_rowid(&conn, TEST_SITE_ID, rowid)
            .expect("Failed to select");

        assert_eq!(retrieved.row_id, rowid);
        assert_eq!(retrieved.site_id, TEST_SITE_ID);
        assert_eq!(retrieved.post, post);
    }

    #[test]
    fn test_repository_select_by_post_id() {
        let conn = setup_test_db();
        let repo = PostRepository;
        let mut post = create_minimal_post();
        post.id = PostId(42);

        // Insert
        repo.insert(&conn, &post, TEST_SITE_ID)
            .expect("Failed to insert");

        // Select by post_id
        let retrieved = repo
            .select_by_post_id(&conn, TEST_SITE_ID, PostId(42))
            .expect("Failed to select by post_id");

        assert_eq!(retrieved.post.id, PostId(42));
        assert_eq!(retrieved.site_id, TEST_SITE_ID);
        assert_eq!(retrieved.post, post);
    }

    #[test]
    fn test_repository_select_by_post_id_not_found() {
        let conn = setup_test_db();
        let repo = PostRepository;

        // Try to select non-existent post
        let result = repo.select_by_post_id(&conn, TEST_SITE_ID, PostId(999));

        assert!(result.is_err());
    }

    #[test]
    fn test_repository_select_by_author() {
        let conn = setup_test_db();
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

        repo.insert(&conn, &post1, TEST_SITE_ID).unwrap();
        repo.insert(&conn, &post2, TEST_SITE_ID).unwrap();
        repo.insert(&conn, &post3, TEST_SITE_ID).unwrap();

        // Select by author
        let author_10_posts = repo.select_by_author(&conn, TEST_SITE_ID, UserId(10)).unwrap();
        assert_eq!(author_10_posts.len(), 2);
        assert!(
            author_10_posts
                .iter()
                .all(|p| p.post.author == Some(UserId(10)))
        );

        let author_20_posts = repo.select_by_author(&conn, TEST_SITE_ID, UserId(20)).unwrap();
        assert_eq!(author_20_posts.len(), 1);
        assert_eq!(author_20_posts[0].post.author, Some(UserId(20)));
    }

    #[test]
    fn test_repository_select_by_status() {
        let conn = setup_test_db();
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

        repo.insert(&conn, &post1, TEST_SITE_ID).unwrap();
        repo.insert(&conn, &post2, TEST_SITE_ID).unwrap();
        repo.insert(&conn, &post3, TEST_SITE_ID).unwrap();

        // Select by status
        let published = repo.select_by_status(&conn, TEST_SITE_ID, "publish").unwrap();
        assert_eq!(published.len(), 2);

        let drafts = repo.select_by_status(&conn, TEST_SITE_ID, "draft").unwrap();
        assert_eq!(drafts.len(), 1);
    }

    #[test]
    fn test_repository_select_all() {
        let conn = setup_test_db();
        let repo = PostRepository;

        // Initially empty
        let all = repo.select_all(&conn, TEST_SITE_ID).unwrap();
        assert_eq!(all.len(), 0);

        // Insert posts
        let mut post1 = create_minimal_post();
        post1.id = PostId(1);
        let mut post2 = create_minimal_post();
        post2.id = PostId(2);

        repo.insert(&conn, &post1, TEST_SITE_ID).unwrap();
        repo.insert(&conn, &post2, TEST_SITE_ID).unwrap();

        // Select all
        let all = repo.select_all(&conn, TEST_SITE_ID).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_repository_count() {
        let conn = setup_test_db();
        let repo = PostRepository;

        assert_eq!(repo.count(&conn, TEST_SITE_ID).unwrap(), 0);

        let mut post1 = create_minimal_post();
        post1.id = PostId(1);
        repo.insert(&conn, &post1, TEST_SITE_ID).unwrap();

        assert_eq!(repo.count(&conn, TEST_SITE_ID).unwrap(), 1);

        let mut post2 = create_minimal_post();
        post2.id = PostId(2);
        repo.insert(&conn, &post2, TEST_SITE_ID).unwrap();

        assert_eq!(repo.count(&conn, TEST_SITE_ID).unwrap(), 2);
    }

    #[test]
    fn test_repository_insert_batch() {
        let mut conn = setup_test_db();
        let repo = PostRepository;

        let mut post1 = create_minimal_post();
        post1.id = PostId(1);
        let mut post2 = create_full_post();
        post2.id = PostId(2);
        let mut post3 = create_minimal_post();
        post3.id = PostId(3);

        let posts = vec![post1, post2, post3];

        // Insert batch
        let rowids = repo.insert_batch(&mut conn, &posts, TEST_SITE_ID).unwrap();
        assert_eq!(rowids.len(), 3);

        // Verify all were inserted
        assert_eq!(repo.count(&conn, TEST_SITE_ID).unwrap(), 3);

        // Verify can retrieve each
        for rowid in rowids {
            repo.select_by_rowid(&conn, TEST_SITE_ID, rowid)
                .expect("Should exist");
        }
    }

    #[test]
    fn test_repository_delete_by_post_id() {
        let conn = setup_test_db();
        let repo = PostRepository;

        let mut post = create_minimal_post();
        post.id = PostId(42);
        repo.insert(&conn, &post, TEST_SITE_ID).unwrap();

        // Verify exists
        repo.select_by_post_id(&conn, TEST_SITE_ID, PostId(42))
            .expect("Post should exist");

        // Delete
        let deleted = repo.delete_by_post_id(&conn, TEST_SITE_ID, PostId(42)).unwrap();
        assert_eq!(deleted, 1);

        // Verify no longer exists
        let result = repo.select_by_post_id(&conn, TEST_SITE_ID, PostId(42));
        assert!(result.is_err());

        // Delete non-existent should return 0
        let deleted = repo.delete_by_post_id(&conn, TEST_SITE_ID, PostId(999)).unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    fn test_repository_upsert_inserts_new_post() {
        let conn = setup_test_db();
        let repo = PostRepository;

        let mut post = create_minimal_post();
        post.id = PostId(100);
        post.status = PostStatus::Draft;

        // Verify post doesn't exist
        assert!(repo.select_by_post_id(&conn, TEST_SITE_ID, PostId(100)).is_err());

        // Upsert should insert
        let rowid = repo.upsert(&conn, TEST_SITE_ID, &post).unwrap();

        // Verify it was inserted
        let retrieved = repo.select_by_post_id(&conn, TEST_SITE_ID, PostId(100)).unwrap();
        assert_eq!(retrieved.row_id, rowid);
        assert_eq!(retrieved.site_id, TEST_SITE_ID);
        assert_eq!(retrieved.post.status, PostStatus::Draft);
    }

    #[test]
    fn test_repository_upsert_updates_existing_post() {
        let conn = setup_test_db();
        let repo = PostRepository;

        // Insert initial post
        let mut post = create_minimal_post();
        post.id = PostId(200);
        post.status = PostStatus::Draft;
        post.slug = "original-slug".to_string();

        let original_rowid = repo.insert(&conn, &post, TEST_SITE_ID).unwrap();

        // Upsert with updated data
        let mut updated_post = create_minimal_post();
        updated_post.id = PostId(200);
        updated_post.status = PostStatus::Publish;
        updated_post.slug = "updated-slug".to_string();

        let new_rowid = repo.upsert(&conn, TEST_SITE_ID, &updated_post).unwrap();

        // Rowid should be the same (it's an update, not delete+insert)
        assert_eq!(original_rowid, new_rowid);

        // Verify the update
        let retrieved = repo.select_by_post_id(&conn, TEST_SITE_ID, PostId(200)).unwrap();
        assert_eq!(retrieved.post.status, PostStatus::Publish);
        assert_eq!(retrieved.post.slug, "updated-slug");

        // Verify only one post exists with this ID
        assert_eq!(repo.count(&conn, TEST_SITE_ID).unwrap(), 1);
    }
}
