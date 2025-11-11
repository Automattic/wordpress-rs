use crate::FullEntityAnyPostWithEditContext;
use std::sync::Arc;
use wp_mobile_cache::{
    DbTable, SqliteDbError, UpdateHook, WpApiCache, context::EditContext,
    db_types::db_site::DbSite, entity::FullEntity, repository::posts::PostRepository,
};

/// Collection of all posts with edit context for a site.
///
/// This collection observes all posts for a site, detecting inserts/updates/deletes
/// using table-level filtering. Unlike `Entity<T>` which tracks individual rows,
/// this collection tracks all rows in a table for a given site.
///
/// The collection is stateless and pragmatic - it doesn't track which specific
/// rows changed, it just knows when *any* row in the table changed and re-queries
/// all data when that happens.
#[derive(uniffi::Object)]
pub struct AllPostsWithEditContextCollection {
    db_site: DbSite,
    cache: Arc<WpApiCache>,
}

impl AllPostsWithEditContextCollection {
    /// Create a new collection for the given site
    pub(crate) fn new(db_site: DbSite, cache: Arc<WpApiCache>) -> Self {
        Self { db_site, cache }
    }
}

#[uniffi::export]
impl AllPostsWithEditContextCollection {
    /// Load all posts from the cache for this site.
    ///
    /// This is an expensive operation that reads from the database each time.
    /// It returns all posts currently stored in the cache, regardless of which
    /// posts triggered the update notification.
    ///
    /// Returns:
    /// - Ok(Vec<FullEntity>) - All posts for this site (may be empty)
    /// - Err(SqliteDbError) if database error occurred
    pub fn load_data(&self) -> Result<Vec<FullEntityAnyPostWithEditContext>, SqliteDbError> {
        let repo = PostRepository::<EditContext>::new();
        self.cache.execute(|connection| {
            repo.select_all(connection, &self.db_site).map(|posts| {
                posts
                    .into_iter()
                    .map(|db_post_full_entity| {
                        FullEntity::new(
                            db_post_full_entity.entity_id,
                            db_post_full_entity.data.post,
                        )
                    })
                    .map(|full_entity| full_entity.into())
                    .collect()
            })
        })
    }

    /// Load all posts from the cache for this site (async version).
    ///
    /// This is an expensive operation that reads from the database each time.
    /// It returns all posts currently stored in the cache, regardless of which
    /// posts triggered the update notification.
    ///
    /// Returns:
    /// - Ok(Vec<FullEntity>) - All posts for this site (may be empty)
    /// - Err(SqliteDbError) if database error occurred
    pub async fn load_data_async(
        &self,
    ) -> Result<Vec<FullEntityAnyPostWithEditContext>, SqliteDbError> {
        // For now, just call the sync version
        // In the future, this could be optimized to run on a background thread
        self.load_data()
    }

    /// Check if a database update is relevant to this collection.
    ///
    /// This uses table-level filtering - any insert, update, or delete to the
    /// posts_edit_context table for this site is considered relevant.
    ///
    /// This is intentionally simple and stateless - we don't track individual
    /// row IDs, we just know "something changed in this table".
    pub fn is_relevant_update(&self, hook: &UpdateHook) -> bool {
        hook.table == DbTable::PostsEditContext
    }
}
