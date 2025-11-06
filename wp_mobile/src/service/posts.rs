use crate::{EntityAnyPostWithEditContext, entity::Entity, entity_error::EntityError};
use std::sync::Arc;
use wp_api::{
    api_client::WpApiClient,
    posts::{AnyPostWithEditContext, PostId},
};
use wp_mobile_cache::{
    WpApiCache, context::EditContext, db_types::db_site::DbSite, repository::posts::PostRepository,
};

/// Service layer for post operations
///
/// Provides a bridge between clients and the underlying network/cache layers.
/// Handles fetching, creating, updating, and deleting posts.
#[derive(uniffi::Object)]
pub struct PostService {
    db_site: Arc<DbSite>,
    api_client: Arc<WpApiClient>,
    cache: Arc<WpApiCache>,
}

impl PostService {
    pub fn new(api_client: Arc<WpApiClient>, db_site: Arc<DbSite>, cache: Arc<WpApiCache>) -> Self {
        Self {
            api_client,
            db_site,
            cache,
        }
    }

    /// Internal helper to read post data from the database
    ///
    /// This is used by both the direct read method and the entity closure.
    fn read_post_from_db_internal(
        cache: &WpApiCache,
        db_site: &DbSite,
        id: PostId,
    ) -> Result<Option<AnyPostWithEditContext>, EntityError> {
        let repo = PostRepository::<EditContext>::new();
        let connection = cache.connection();

        repo.select_by_post_id(&*connection, db_site, id)
            .map(|opt| opt.map(|db_post| db_post.post))
            .map_err(|e| e.into())
    }

    /// Read post data directly from the database
    ///
    /// This bypasses the entity layer for direct access to cached post data.
    /// Can be made public in the future if needed.
    pub(crate) fn read_post_from_db(
        &self,
        id: PostId,
    ) -> Result<Option<AnyPostWithEditContext>, EntityError> {
        Self::read_post_from_db_internal(&self.cache, &self.db_site, id)
    }
}

// TODO: We probably want to implement an impl block that returns Entity<T> types and have a
// attribute macro to generate the uniffi exported counterpart, as it'd be easier to work with the
// generic type in Rust than the concrete wrapper type such as EntityAnyPostWithEditContext

#[uniffi::export]
impl PostService {
    /// Get an entity handle for a specific post
    ///
    /// Returns an entity that can be used to read post data.
    /// The entity is lightweight - it doesn't fetch data until you call load_data() on it.
    pub fn get_entity(&self, id: PostId) -> EntityAnyPostWithEditContext {
        let cache = self.cache.clone();
        let db_site = self.db_site.clone();
        let id_val = id.0;

        // Get table name and rowid for relevance checking
        let table_name = PostRepository::<EditContext>::table_name();
        let repo = PostRepository::<EditContext>::new();
        let connection = self.cache.connection();
        let db_post = repo
            .select_by_post_id(&*connection, &self.db_site, id)
            .expect("Failed to read post from database");
        drop(connection);

        let rowid = db_post.as_ref().map(|p| p.row_id.0 as i64).unwrap_or(0);

        Entity::<AnyPostWithEditContext>::new(
            id.0,
            Box::new(move || Self::read_post_from_db_internal(&cache, &db_site, PostId(id_val))),
            Box::new(move |hook: &wp_mobile_cache::UpdateHook| {
                hook.table_name == table_name && hook.row_id == rowid
            }),
        )
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::mock_api_client;
    use rstest::*;
    use rusqlite::Connection;
    use wp_mobile_cache::{
        MigrationManager, WpApiCache,
        db_types::self_hosted_site::SelfHostedSite,
        repository::{posts::PostRepository, sites::SiteRepository},
        test_fixtures::posts::PostBuilder,
    };

    #[rstest]
    fn test_read_post_from_db_returns_cached_post(post_service_ctx: PostServiceTestContext) {
        // Setup: Insert test post into cache
        let test_post = insert_test_post(&post_service_ctx);

        // Test: Read post from database
        let result = post_service_ctx
            .post_service
            .read_post_from_db(test_post.id)
            .expect("Database read should succeed");

        // Assert: Post was found and matches what we inserted
        let retrieved_post = result.expect("Post should be found in cache");
        test_post.assert_matches(&retrieved_post);
    }

    #[rstest]
    fn test_get_entity_load_data_returns_cached_post(post_service_ctx: PostServiceTestContext) {
        // Setup: Insert test post into cache
        let test_post = insert_test_post(&post_service_ctx);

        // Test: Get entity and load data
        let entity = post_service_ctx.post_service.get_entity(test_post.id);
        let result = entity.load_data().expect("Database read should succeed");

        // Assert: Post was found and matches what we inserted
        let retrieved_post = result.expect("Post should be found in cache");
        test_post.assert_matches(&retrieved_post);
    }

    /// Test helper that encapsulates a test post with its assertion logic
    struct TestPost {
        id: PostId,
        title: String,
        slug: String,
    }

    impl TestPost {
        /// Assert that a retrieved post matches the expected values
        fn assert_matches(&self, post: &AnyPostWithEditContext) {
            assert_eq!(post.id, self.id);
            assert_eq!(post.title.rendered, self.title);
            assert_eq!(post.slug, self.slug);
        }
    }

    /// Helper function to insert a test post into the cache
    ///
    /// Creates a test post with predefined values and inserts it into the database.
    /// Returns a TestPost that can be used to assert the retrieved data matches what was inserted.
    /// This common setup is used by multiple tests to showcase similarities between
    /// direct database reads and entity-based reads.
    fn insert_test_post(ctx: &PostServiceTestContext) -> TestPost {
        let test_post = TestPost {
            id: PostId(42),
            title: "Test Post".to_string(),
            slug: "test-post".to_string(),
        };

        let post = PostBuilder::minimal()
            .with_id(test_post.id.0)
            .with_title(&test_post.title)
            .with_slug(&test_post.slug)
            .build();

        let post_repo = PostRepository::<EditContext>::new();
        let mut conn = ctx.cache.connection();
        post_repo
            .upsert(&mut *conn, &ctx.db_site, &post)
            .expect("Post insert should succeed");
        drop(conn); // Release the connection lock

        test_post
    }

    /// Test context bundling PostService with database and site setup
    pub struct PostServiceTestContext {
        pub post_service: PostService,
        pub db_site: Arc<DbSite>,
        pub cache: Arc<WpApiCache>,
    }

    /// rstest fixture providing a PostService with in-memory database
    ///
    /// Sets up an in-memory SQLite database with migrations, creates a test site,
    /// and returns a PostService instance ready for testing.
    ///
    /// # Example
    ///
    /// ```rust
    /// #[rstest]
    /// fn test_something(post_service_ctx: PostServiceTestContext) {
    ///     let result = post_service_ctx.post_service.read_post_from_db(PostId(1));
    ///     // ...
    /// }
    /// ```
    #[fixture]
    fn post_service_ctx(mock_api_client: Arc<WpApiClient>) -> PostServiceTestContext {
        // Setup: Create in-memory database with migrations
        let mut conn = Connection::open_in_memory()
            .expect("Failed to create in-memory database");
        let mut migration_manager = MigrationManager::new(&conn)
            .expect("Failed to create migration manager");
        migration_manager
            .perform_migrations()
            .expect("Migrations should succeed");

        // Setup: Create test site
        let site_repo = SiteRepository;
        let self_hosted_site = SelfHostedSite {
            url: "https://test.local".to_string(),
            api_root: "https://test.local/wp-json".to_string(),
        };
        let (db_site, _) = site_repo
            .upsert_self_hosted_site(&mut conn, &self_hosted_site)
            .expect("Site creation should succeed");

        // Setup: Create PostService with cache
        let cache = Arc::new(WpApiCache::from(conn));
        let db_site_arc = Arc::new(db_site);
        let post_service = PostService::new(mock_api_client, db_site_arc.clone(), cache.clone());

        PostServiceTestContext {
            post_service,
            db_site: db_site_arc,
            cache,
        }
    }
}
