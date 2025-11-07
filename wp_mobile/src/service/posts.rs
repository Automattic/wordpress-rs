use crate::EntityAnyPostWithEditContext;
use std::sync::Arc;
use wp_api::{
    api_client::WpApiClient,
    posts::{AnyPostWithEditContext, PostId},
};
use wp_mobile_cache::{
    WpApiCache,
    context::EditContext,
    db_types::db_site::DbSite,
    entity::{Entity, EntityId, FullEntity},
    repository::posts::PostRepository,
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
}

// TODO: We probably want to implement an impl block that returns Entity<T> types and have a
// attribute macro to generate the uniffi exported counterpart, as it'd be easier to work with the
// generic type in Rust than the concrete wrapper type such as EntityAnyPostWithEditContext

#[uniffi::export]
impl PostService {
    fn create_temp_post(&self, id: PostId) -> AnyPostWithEditContext {
        use wp_api::posts::{
            PostContentWithEditContext, PostGuidWithEditContext, PostStatus,
            PostTitleWithEditContext,
        };

        AnyPostWithEditContext {
            id,
            date: "2025-01-01T00:00:00".to_string(),
            date_gmt: "2025-01-01T00:00:00Z".parse().unwrap(),
            guid: PostGuidWithEditContext {
                raw: None,
                rendered: format!("https://example.com/?p={}", id.0),
            },
            link: format!("https://example.com/test-post-{}", id.0),
            modified: "2025-01-01T00:00:00".to_string(),
            modified_gmt: "2025-01-01T00:00:00Z".parse().unwrap(),
            slug: format!("test-post-{}", id.0),
            status: PostStatus::Publish,
            post_type: "post".to_string(),
            password: "".to_string(),
            permalink_template: None,
            generated_slug: None,
            title: PostTitleWithEditContext {
                raw: None,
                rendered: "Test Post".to_string(),
            },
            content: PostContentWithEditContext {
                raw: None,
                rendered: "<p>Test content</p>".to_string(),
                protected: None,
                block_version: None,
            },
            author: None,
            excerpt: None,
            featured_media: None,
            comment_status: None,
            ping_status: None,
            format: None,
            meta: None,
            sticky: None,
            template: "".to_string(),
            categories: None,
            tags: None,
            parent: None,
            menu_order: None,
        }
    }

    /// TEMPORARY: Insert a mock post for testing purposes
    ///
    /// This is a temporary method to test the observer pattern without needing
    /// the full API client stack. Should be removed once proper data insertion is available.
    pub fn insert_mock_post_for_testing(&self, id: PostId, title: String) -> EntityId {
        let mut post = self.create_temp_post(id);
        post.title.rendered = title;

        let repo = PostRepository::<EditContext>::new();
        let mut conn = self.cache.connection();
        repo.upsert(&mut *conn, &self.db_site, &post)
            .expect("Failed to insert mock post")
    }

    /// TEMPORARY: Update a mock post for testing purposes
    ///
    /// Updates an existing post's title. Used for testing the observer pattern.
    pub fn update_mock_post_for_testing(&self, id: PostId, new_title: String) {
        let repo = PostRepository::<EditContext>::new();
        let mut conn = self.cache.connection();
        let mut post = repo
            .select_by_post_id(&*conn, &self.db_site, id)
            .expect("Failed to read post")
            .expect("Post not found")
            .data
            .post;
        post.title.rendered = new_title;
        repo.upsert(&mut *conn, &self.db_site, &post)
            .expect("Failed to update mock post");
    }

    /// Get an entity handle using an EntityId
    ///
    /// Returns an entity that can be used to read post data with full edit context.
    /// The entity is lightweight - it doesn't fetch data until you call load_data() on it.
    ///
    /// The EntityId should come from repository results (e.g., select_by_post_id).
    pub fn get_entity_with_edit_context(
        &self,
        entity_id: Arc<EntityId>,
    ) -> EntityAnyPostWithEditContext {
        let cache = self.cache.clone();
        let entity_id = *entity_id;

        Entity::<AnyPostWithEditContext>::new(
            entity_id,
            Box::new(move || {
                let repo = PostRepository::<EditContext>::new();
                let connection = cache.connection();

                repo.select_by_entity_id(&*connection, &entity_id)
                    .map(|opt| {
                        opt.map(|db_post_full_entity| {
                            FullEntity::new(
                                db_post_full_entity.entity_id,
                                db_post_full_entity.data.post,
                            )
                        })
                    })
                    .map_err(|e| e.into())
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
        HookAction, MigrationManager, UpdateHook, WpApiCache,
        db_types::self_hosted_site::SelfHostedSite,
        repository::{posts::PostRepository, sites::SiteRepository},
        test_fixtures::posts::PostBuilder,
    };

    #[rstest]
    fn test_get_entity_load_data_returns_cached_post(post_service_ctx: PostServiceTestContext) {
        // Setup: Insert test post into cache
        let test_post = insert_test_post(&post_service_ctx);

        // Test: Get EntityId from repository, then create entity
        let repo = PostRepository::<EditContext>::new();
        let connection = post_service_ctx.cache.connection();
        let full_entity_from_repo = repo
            .select_by_post_id(&*connection, &post_service_ctx.db_site, test_post.id)
            .expect("Database read should succeed")
            .expect("Post should exist");
        let entity_id = full_entity_from_repo.entity_id;
        drop(connection);

        let entity = post_service_ctx
            .post_service
            .get_entity_with_edit_context(entity_id);
        let result = entity.load_data().expect("Database read should succeed");

        // Assert: Post was found and matches what we inserted
        let full_entity = result.expect("Post should be found in cache");
        test_post.assert_matches(&full_entity.data);
    }

    #[rstest]
    fn test_entity_is_relevant_update_matches_correct_updates(
        post_service_ctx: PostServiceTestContext,
    ) {
        // Setup: Insert test post
        let test_post = insert_test_post(&post_service_ctx);

        // Get EntityId from repository
        let repo = PostRepository::<EditContext>::new();
        let connection = post_service_ctx.cache.connection();
        let full_entity_from_repo = repo
            .select_by_post_id(&*connection, &post_service_ctx.db_site, test_post.id)
            .expect("Should read post")
            .expect("Post should exist");
        let entity_id = full_entity_from_repo.entity_id.clone();
        drop(connection);

        let entity = post_service_ctx
            .post_service
            .get_entity_with_edit_context(entity_id.clone());

        // Get the table name and rowid from the entity_id
        let table_name = entity_id.table_name;
        let rowid = entity_id.rowid.0 as i64;

        // Test: Create UpdateHook that matches this entity
        let matching_hook = UpdateHook {
            action: HookAction::Update,
            db_name: "main".to_string(),
            table_name: table_name.to_string(),
            row_id: rowid,
        };

        // Assert: Entity should recognize this update as relevant
        assert!(
            entity.0.is_relevant_update(&matching_hook),
            "Entity should match updates with same table and rowid"
        );

        // Test: Create UpdateHook with different table
        let wrong_table_hook = UpdateHook {
            action: HookAction::Update,
            db_name: "main".to_string(),
            table_name: "wrong_table".to_string(),
            row_id: rowid,
        };

        // Assert: Entity should not match updates from different table
        assert!(
            !entity.0.is_relevant_update(&wrong_table_hook),
            "Entity should not match updates from different table"
        );

        // Test: Create UpdateHook with different rowid
        let wrong_rowid_hook = UpdateHook {
            action: HookAction::Update,
            db_name: "main".to_string(),
            table_name: table_name.to_string(),
            row_id: rowid + 1,
        };

        // Assert: Entity should not match updates for different row
        assert!(
            !entity.0.is_relevant_update(&wrong_rowid_hook),
            "Entity should not match updates for different rowid"
        );
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
        let mut conn = Connection::open_in_memory().expect("Failed to create in-memory database");
        let mut migration_manager =
            MigrationManager::new(&conn).expect("Failed to create migration manager");
        migration_manager
            .perform_migrations()
            .expect("Migrations should succeed");

        // Setup: Create test site
        let site_repo = SiteRepository;
        let self_hosted_site = SelfHostedSite {
            url: "https://test.local".to_string(),
            api_root: "https://test.local/wp-json".to_string(),
        };
        let db_site = site_repo
            .upsert_self_hosted_site(&mut conn, &self_hosted_site)
            .expect("Site creation should succeed")
            .db_site;

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
