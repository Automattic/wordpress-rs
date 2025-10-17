use crate::{
    SqliteDbError,
    mappings::{InsertIntoDb, TryFromDbRow, posts::DbAnyPostWithEditContext},
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
    /// Select a post by its SQLite rowid (returns wrapper with rowid).
    ///
    /// Returns an error if no post with the given rowid exists.
    pub fn select_by_rowid(
        &self,
        executor: &impl QueryExecutor,
        rowid: i64,
    ) -> Result<DbAnyPostWithEditContext, SqliteDbError> {
        let sql = "SELECT * FROM posts_edit_context WHERE rowid = ?";
        let mut stmt = executor.prepare(sql)?;
        stmt.query_row([rowid], |row| {
            DbAnyPostWithEditContext::try_from_row(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })
        .map_err(SqliteDbError::from)
    }

    /// Select all posts from the table (returns wrappers with rowids).
    ///
    /// Returns an empty vector if the table is empty.
    pub fn select_all(
        &self,
        executor: &impl QueryExecutor,
    ) -> Result<Vec<DbAnyPostWithEditContext>, SqliteDbError> {
        let sql = "SELECT * FROM posts_edit_context";
        let mut stmt = executor.prepare(sql)?;
        let rows = stmt.query_map([], |row| {
            DbAnyPostWithEditContext::try_from_row(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(SqliteDbError::from)
    }

    /// Select a post by its WordPress post ID (returns wrapper with rowid).
    ///
    /// This is different from `select_by_rowid` which uses the SQLite rowid.
    /// The post_id is the WordPress post ID from the REST API.
    ///
    /// Returns an error if no post with the given ID exists.
    pub fn select_by_post_id(
        &self,
        executor: &impl QueryExecutor,
        post_id: PostId,
    ) -> Result<DbAnyPostWithEditContext, SqliteDbError> {
        let sql = "SELECT * FROM posts_edit_context WHERE id = ?";
        let mut stmt = executor.prepare(sql)?;
        stmt.query_row([post_id.0], |row| {
            DbAnyPostWithEditContext::try_from_row(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })
        .map_err(SqliteDbError::from)
    }

    /// Select posts by author user ID (returns wrappers with rowids).
    ///
    /// Returns an empty vector if no posts by the given author exist.
    pub fn select_by_author(
        &self,
        executor: &impl QueryExecutor,
        author_id: wp_api::users::UserId,
    ) -> Result<Vec<DbAnyPostWithEditContext>, SqliteDbError> {
        let sql = "SELECT * FROM posts_edit_context WHERE author = ?";
        let mut stmt = executor.prepare(sql)?;
        let rows = stmt.query_map([author_id.0], |row| {
            DbAnyPostWithEditContext::try_from_row(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(SqliteDbError::from)
    }

    /// Select posts by status (e.g., "publish", "draft").
    ///
    /// Returns an empty vector if no posts with the given status exist.
    pub fn select_by_status(
        &self,
        executor: &impl QueryExecutor,
        status: &str,
    ) -> Result<Vec<DbAnyPostWithEditContext>, SqliteDbError> {
        let sql = "SELECT * FROM posts_edit_context WHERE status = ?";
        let mut stmt = executor.prepare(sql)?;
        let rows = stmt.query_map([status], |row| {
            DbAnyPostWithEditContext::try_from_row(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(SqliteDbError::from)
    }

    /// Delete a post by its WordPress post ID.
    ///
    /// Returns the number of rows deleted (0 or 1).
    pub fn delete_by_post_id(
        &self,
        executor: &impl QueryExecutor,
        post_id: PostId,
    ) -> Result<usize, SqliteDbError> {
        let sql = "DELETE FROM posts_edit_context WHERE id = ?";
        executor.execute(sql, [post_id.0])
    }

    /// Update an existing post by its WordPress post ID.
    ///
    /// Returns the number of rows affected (should be 1 if successful, 0 if not found).
    pub fn update_by_post_id(
        &self,
        conn: &rusqlite::Connection,
        post: &AnyPostWithEditContext,
    ) -> Result<usize, SqliteDbError> {
        // Delete the old post and insert the new one
        // This is simpler than a full UPDATE statement with all fields
        let deleted = self.delete_by_post_id(conn, post.id)?;
        if deleted > 0 {
            InsertIntoDb::insert_into_db(post, conn)?;
            Ok(1)
        } else {
            Ok(0)
        }
    }

    /// Get the total count of posts in the database.
    pub fn count(&self, executor: &impl QueryExecutor) -> Result<i64, SqliteDbError> {
        let sql = "SELECT COUNT(*) FROM posts_edit_context";
        let mut stmt = executor.prepare(sql)?;
        stmt.query_row([], |row| row.get(0))
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

    #[test]
    fn test_repository_insert_and_select_by_rowid() {
        let conn = setup_test_db();
        let repo = PostRepository;
        let post = create_minimal_post();

        // Insert using repository
        let rowid = repo.insert(&conn, &post).expect("Failed to insert");

        // Select by rowid
        let retrieved = repo
            .select_by_rowid(&conn, rowid)
            .expect("Failed to select");

        assert_eq!(retrieved.row_id, rowid);
        assert_eq!(retrieved.post, post);
    }

    #[test]
    fn test_repository_select_by_post_id() {
        let conn = setup_test_db();
        let repo = PostRepository;
        let mut post = create_minimal_post();
        post.id = PostId(42);

        // Insert
        repo.insert(&conn, &post).expect("Failed to insert");

        // Select by post_id
        let retrieved = repo
            .select_by_post_id(&conn, PostId(42))
            .expect("Failed to select by post_id");

        assert_eq!(retrieved.post.id, PostId(42));
        assert_eq!(retrieved.post, post);
    }

    #[test]
    fn test_repository_select_by_post_id_not_found() {
        let conn = setup_test_db();
        let repo = PostRepository;

        // Try to select non-existent post
        let result = repo.select_by_post_id(&conn, PostId(999));

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

        repo.insert(&conn, &post1).unwrap();
        repo.insert(&conn, &post2).unwrap();
        repo.insert(&conn, &post3).unwrap();

        // Select by author
        let author_10_posts = repo.select_by_author(&conn, UserId(10)).unwrap();
        assert_eq!(author_10_posts.len(), 2);
        assert!(
            author_10_posts
                .iter()
                .all(|p| p.post.author == Some(UserId(10)))
        );

        let author_20_posts = repo.select_by_author(&conn, UserId(20)).unwrap();
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

        repo.insert(&conn, &post1).unwrap();
        repo.insert(&conn, &post2).unwrap();
        repo.insert(&conn, &post3).unwrap();

        // Select by status
        let published = repo.select_by_status(&conn, "publish").unwrap();
        assert_eq!(published.len(), 2);

        let drafts = repo.select_by_status(&conn, "draft").unwrap();
        assert_eq!(drafts.len(), 1);
    }

    #[test]
    fn test_repository_select_all() {
        let conn = setup_test_db();
        let repo = PostRepository;

        // Initially empty
        let all = repo.select_all(&conn).unwrap();
        assert_eq!(all.len(), 0);

        // Insert posts
        let mut post1 = create_minimal_post();
        post1.id = PostId(1);
        let mut post2 = create_minimal_post();
        post2.id = PostId(2);

        repo.insert(&conn, &post1).unwrap();
        repo.insert(&conn, &post2).unwrap();

        // Select all
        let all = repo.select_all(&conn).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_repository_count() {
        let conn = setup_test_db();
        let repo = PostRepository;

        assert_eq!(repo.count(&conn).unwrap(), 0);

        let mut post1 = create_minimal_post();
        post1.id = PostId(1);
        repo.insert(&conn, &post1).unwrap();

        assert_eq!(repo.count(&conn).unwrap(), 1);

        let mut post2 = create_minimal_post();
        post2.id = PostId(2);
        repo.insert(&conn, &post2).unwrap();

        assert_eq!(repo.count(&conn).unwrap(), 2);
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
        let rowids = repo.insert_batch(&mut conn, &posts).unwrap();
        assert_eq!(rowids.len(), 3);

        // Verify all were inserted
        assert_eq!(repo.count(&conn).unwrap(), 3);

        // Verify can retrieve each
        for rowid in rowids {
            repo.select_by_rowid(&conn, rowid).expect("Should exist");
        }
    }

    #[test]
    fn test_repository_delete_by_post_id() {
        let conn = setup_test_db();
        let repo = PostRepository;

        let mut post = create_minimal_post();
        post.id = PostId(42);
        repo.insert(&conn, &post).unwrap();

        // Verify exists
        repo.select_by_post_id(&conn, PostId(42))
            .expect("Post should exist");

        // Delete
        let deleted = repo.delete_by_post_id(&conn, PostId(42)).unwrap();
        assert_eq!(deleted, 1);

        // Verify no longer exists
        let result = repo.select_by_post_id(&conn, PostId(42));
        assert!(result.is_err());

        // Delete non-existent should return 0
        let deleted = repo.delete_by_post_id(&conn, PostId(999)).unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    fn test_repository_update_by_post_id() {
        let conn = setup_test_db();
        let repo = PostRepository;

        let mut post = create_minimal_post();
        post.id = PostId(42);
        post.status = PostStatus::Draft;

        repo.insert(&conn, &post).unwrap();

        // Create an updated version of the post
        let mut updated_post = create_minimal_post();
        updated_post.id = PostId(42);
        updated_post.status = PostStatus::Publish;
        updated_post.slug = "updated-slug".to_string();

        let affected = repo.update_by_post_id(&conn, &updated_post).unwrap();
        assert_eq!(affected, 1);

        // Verify update
        let retrieved = repo.select_by_post_id(&conn, PostId(42)).unwrap();
        assert_eq!(retrieved.post.status, PostStatus::Publish);
        assert_eq!(retrieved.post.slug, "updated-slug");

        // Update non-existent should return 0
        let mut non_existent = create_minimal_post();
        non_existent.id = PostId(999);
        let affected = repo.update_by_post_id(&conn, &non_existent).unwrap();
        assert_eq!(affected, 0);
    }
}
