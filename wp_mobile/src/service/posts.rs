use crate::{
    AllAnyPostWithEditContextCollection, EntityAnyPostWithEditContext,
    PostCollectionWithEditContext,
    collection::{FetchError, FetchResult, StatelessCollection, post_collection::PostCollection},
    filters::AnyPostFilter,
    sync::{EntityMetadata, MetadataFetchResult},
};
use std::sync::Arc;
use wp_api::{
    api_client::WpApiClient,
    posts::{AnyPostWithEditContext, PostId, PostListParams, SparseAnyPostFieldWithEditContext},
    request::endpoint::posts_endpoint::PostEndpointType,
};
use wp_mobile_cache::{
    DbTable, WpApiCache,
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

    /// Fetch posts from network and save to cache
    ///
    /// This is the core networking primitive. It:
    /// 1. Converts filter to API parameters
    /// 2. Makes network request via WpApiClient
    /// 3. Upserts posts to database via repository
    /// 4. Returns entity IDs and pagination info
    ///
    /// # Arguments
    /// * `filter` - Post filter criteria
    /// * `page` - Page number to fetch (1-indexed)
    /// * `per_page` - Number of posts per page
    ///
    /// # Returns
    /// - `Ok(FetchResult)` with entity IDs of fetched posts
    /// - `Err(FetchError)` if network or database error occurs
    ///
    /// # Database Updates
    /// Successful fetch triggers database update hooks, which notify
    /// any observers watching the relevant tables.
    ///
    /// # Note
    /// This is an async function because network operations are async.
    /// Platform-specific wrappers (Kotlin/Swift) will need to handle
    /// the async bridge.
    pub async fn fetch_posts_page(
        &self,
        filter: &AnyPostFilter,
        page: u32,
        per_page: u32,
    ) -> Result<FetchResult, FetchError> {
        // Convert filter to API params
        let mut params = filter.to_list_params();
        params.page = Some(page);
        params.per_page = Some(per_page);

        // Make network request
        let response = self
            .api_client
            .posts()
            .list_with_edit_context(&PostEndpointType::Posts, &params)
            .await?;

        // Upsert to database and collect entity IDs
        let entity_ids = self.cache.execute(|conn| {
            let repo = PostRepository::<EditContext>::new();

            response
                .data
                .iter()
                .map(|post| {
                    repo.upsert(conn, &self.db_site, post)
                        .map_err(|e| FetchError::Database {
                            err_message: e.to_string(),
                        })
                })
                .collect::<Result<Vec<_>, _>>()
        })?;

        Ok(FetchResult {
            entity_ids,
            total_items: response.header_map.wp_total().map(|n| n as i64),
            total_pages: response.header_map.wp_total_pages(),
            current_page: page,
        })
    }

    /// Fetch only metadata (id + modified_gmt) for a page of posts.
    ///
    /// This is a lightweight fetch that returns just enough information to:
    /// 1. Define list structure (order and IDs)
    /// 2. Determine which posts need full fetching (missing or stale)
    ///
    /// Unlike `fetch_posts_page`, this does NOT upsert to the database.
    /// The metadata is used transiently to drive selective sync.
    ///
    /// # Arguments
    /// * `filter` - Post filter criteria
    /// * `page` - Page number to fetch (1-indexed)
    /// * `per_page` - Number of posts per page
    ///
    /// # Returns
    /// - `Ok(MetadataFetchResult)` with post IDs and modification times
    /// - `Err(FetchError)` if network error occurs
    pub async fn fetch_posts_metadata(
        &self,
        filter: &AnyPostFilter,
        page: u32,
        per_page: u32,
    ) -> Result<MetadataFetchResult<PostId>, FetchError> {
        let mut params = filter.to_list_params();
        params.page = Some(page);
        params.per_page = Some(per_page);

        let response = self
            .api_client
            .posts()
            .filter_list_with_edit_context(
                &PostEndpointType::Posts,
                &params,
                &[
                    SparseAnyPostFieldWithEditContext::Id,
                    SparseAnyPostFieldWithEditContext::ModifiedGmt,
                ],
            )
            .await?;

        // Map sparse posts to EntityMetadata, filtering out any with missing fields
        let metadata: Vec<EntityMetadata<PostId>> = response
            .data
            .into_iter()
            .filter_map(|sparse| Some(EntityMetadata::new(sparse.id?, sparse.modified_gmt?)))
            .collect();

        Ok(MetadataFetchResult {
            metadata,
            total_items: response.header_map.wp_total().map(|n| n as i64),
            total_pages: response.header_map.wp_total_pages(),
            current_page: page,
        })
    }

    /// Fetch full post data for specific post IDs and save to cache.
    ///
    /// This is used for selective sync - fetching only the posts that are
    /// missing or stale in the cache. Uses the `include` parameter to batch
    /// multiple posts in a single request.
    ///
    /// # Arguments
    /// * `ids` - Post IDs to fetch
    ///
    /// # Returns
    /// - `Ok(Vec<EntityId>)` with entity IDs of fetched posts
    /// - `Err(FetchError)` if network or database error occurs
    ///
    /// # Note
    /// If `ids` is empty, returns an empty Vec without making a network request.
    pub async fn fetch_posts_by_ids(&self, ids: Vec<PostId>) -> Result<Vec<EntityId>, FetchError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let params = PostListParams {
            include: ids,
            // Ensure we get all requested posts regardless of default per_page
            per_page: Some(100),
            ..Default::default()
        };

        let response = self
            .api_client
            .posts()
            .list_with_edit_context(&PostEndpointType::Posts, &params)
            .await?;

        // Upsert to database and collect entity IDs
        let entity_ids = self.cache.execute(|conn| {
            let repo = PostRepository::<EditContext>::new();

            response
                .data
                .iter()
                .map(|post| {
                    repo.upsert(conn, &self.db_site, post)
                        .map_err(|e| FetchError::Database {
                            err_message: e.to_string(),
                        })
                })
                .collect::<Result<Vec<_>, _>>()
        })?;

        Ok(entity_ids)
    }
}

#[uniffi::export]
impl PostService {
    /// Get an entity handle using an EntityId
    ///
    /// Returns an entity that can be used to read post data with full edit context.
    /// The entity is lightweight - it doesn't fetch data until you call load_data() on it.
    ///
    /// The EntityId should come from repository results (e.g., select_by_post_id).
    pub fn get_entity_with_edit_context(
        &self,
        entity_id: EntityId,
    ) -> EntityAnyPostWithEditContext {
        let cache = self.cache.clone();

        Entity::<AnyPostWithEditContext>::new(
            entity_id,
            Box::new(move || {
                let repo = PostRepository::<EditContext>::new();

                cache
                    .execute(|connection| repo.select_by_entity_id(connection, &entity_id))
                    .map(|opt| {
                        opt.map(|db_post_full_entity| {
                            FullEntity::new(
                                db_post_full_entity.entity_id,
                                db_post_full_entity.data.post,
                            )
                        })
                    })
            }),
        )
        .into()
    }

    /// Get the total count of posts for this site
    ///
    /// Returns the number of posts stored in the cache for this site.
    pub fn count_edit_context(&self) -> Result<i64, wp_mobile_cache::SqliteDbError> {
        let repo = PostRepository::<EditContext>::new();
        self.cache
            .execute(|connection| repo.count(connection, &self.db_site))
    }

    /// Delete a post by its EntityId
    ///
    /// Returns the number of rows deleted (0 or 1).
    /// Automatically deletes associated term relationships.
    ///
    /// # Arguments
    /// * `entity_id` - The EntityId of the post to delete
    ///
    /// # Returns
    /// - `Ok(1)` if the post was deleted
    /// - `Ok(0)` if the post doesn't exist
    /// - `Err` if there was a database error
    pub fn delete_by_entity_id(
        &self,
        entity_id: &EntityId,
    ) -> Result<u64, wp_mobile_cache::SqliteDbError> {
        let repo = PostRepository::<EditContext>::new();
        self.cache.execute(|connection| {
            repo.delete_by_entity_id(connection, entity_id)
                .map(|n| n as u64)
        })
    }

    /// Delete a post by its WordPress post ID
    ///
    /// Returns the number of rows deleted (0 or 1).
    /// Automatically deletes associated term relationships.
    ///
    /// # Arguments
    /// * `post_id` - The WordPress post ID to delete
    ///
    /// # Returns
    /// - `Ok(1)` if the post was deleted
    /// - `Ok(0)` if the post doesn't exist
    /// - `Err` if there was a database error
    pub fn delete_by_post_id(
        &self,
        post_id: wp_api::posts::PostId,
    ) -> Result<u64, wp_mobile_cache::SqliteDbError> {
        let repo = PostRepository::<EditContext>::new();
        self.cache.execute(|connection| {
            repo.delete_by_post_id(connection, &self.db_site, post_id)
                .map(|n| n as u64)
        })
    }

    /// Create a filtered post collection with edit context
    ///
    /// Returns a collection that:
    /// - Filters posts based on the provided filter criteria
    /// - Supports network fetching via fetch_page()
    /// - Monitors database changes and provides load_data() for cache access
    ///
    /// # Arguments
    /// * `filter` - Filter criteria for posts (status, etc.)
    ///
    /// # Example (Kotlin)
    /// ```kotlin
    /// val filter = AnyPostFilter(status = PostStatus.DRAFT)
    /// val collection = postService.createPostCollectionWithEditContext(filter)
    ///
    /// // Fetch from network
    /// val result = collection.fetchPage(1u, 10u)
    ///
    /// // Load from cache
    /// val posts = collection.loadData()
    /// ```
    pub fn create_post_collection_with_edit_context(
        self: &Arc<Self>,
        filter: AnyPostFilter,
    ) -> PostCollectionWithEditContext {
        let cache = self.cache.clone();
        let db_site = *self.db_site;
        let filter_clone = filter.clone();

        // Create StatelessCollection with filtering
        let stateless_collection = StatelessCollection::new(
            vec![DbTable::PostsEditContext, DbTable::TermRelationships],
            Box::new(move || {
                let repo = PostRepository::<EditContext>::new();
                cache.execute(|connection| {
                    repo.select_by_filter(connection, &db_site, filter_clone.status.as_ref())
                        .map(|posts| {
                            posts
                                .into_iter()
                                .map(|db_post_full_entity| {
                                    FullEntity::new(
                                        db_post_full_entity.entity_id,
                                        db_post_full_entity.data.post,
                                    )
                                })
                                .collect()
                        })
                })
            }),
        );

        PostCollection::new(filter, stateless_collection, self.clone()).into()
    }

    /// Get a collection of all posts with edit context for this site.
    ///
    /// Returns a collection that can be used to observe all posts for this site.
    /// The collection monitors both the posts table and term relationships table -
    /// any insert, update, or delete to either table will trigger observers.
    ///
    /// Unlike individual entities, the collection re-queries all posts when any
    /// relevant change occurs.
    pub fn get_all_posts_with_edit_context(&self) -> AllAnyPostWithEditContextCollection {
        let cache = self.cache.clone();
        let db_site = *self.db_site;

        StatelessCollection::new(
            vec![
                wp_mobile_cache::DbTable::PostsEditContext,
                wp_mobile_cache::DbTable::TermRelationships,
            ],
            Box::new(move || {
                let repo = PostRepository::<EditContext>::new();
                cache.execute(|connection| {
                    repo.select_all(connection, &db_site).map(|posts| {
                        posts
                            .into_iter()
                            .map(|db_post_full_entity| {
                                FullEntity::new(
                                    db_post_full_entity.entity_id,
                                    db_post_full_entity.data.post,
                                )
                            })
                            .collect()
                    })
                })
            }),
        )
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::mock_api_client;
    use rstest::*;
    use rusqlite::Connection;
    use wp_api::posts::PostId;
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
        let entity_id = post_service_ctx
            .cache
            .execute(|conn| {
                let repo = PostRepository::<EditContext>::new();
                repo.select_by_post_id(conn, &post_service_ctx.db_site, test_post.id)
                    .map(|opt| opt.map(|full_entity| *full_entity.entity_id))
            })
            .expect("Database read should succeed")
            .expect("Post should exist");

        let entity = post_service_ctx
            .post_service
            .get_entity_with_edit_context(entity_id);
        // Use the internal Entity's sync load_data for testing
        let result = entity.0.load_data().expect("Database read should succeed");

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
        let entity_id = post_service_ctx
            .cache
            .execute(|conn| {
                let repo = PostRepository::<EditContext>::new();
                repo.select_by_post_id(conn, &post_service_ctx.db_site, test_post.id)
                    .map(|opt| opt.map(|full_entity| *full_entity.entity_id))
            })
            .expect("Database read should succeed")
            .expect("Post should exist");

        let entity = post_service_ctx
            .post_service
            .get_entity_with_edit_context(entity_id);

        // Get the table and rowid from the entity_id
        let table = entity_id.table;
        let rowid = entity_id.rowid.0;

        // Test: Create UpdateHook that matches this entity
        let matching_hook = UpdateHook {
            action: HookAction::Update,
            db_name: "main".to_string(),
            table,
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
            table: wp_mobile_cache::DbTable::PostsViewContext, // Different table
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
            table,
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

        ctx.cache
            .execute(|conn| {
                let post_repo = PostRepository::<EditContext>::new();
                post_repo.upsert(conn, &ctx.db_site, &post)
            })
            .expect("Post insert should succeed");

        test_post
    }

    /// Test context bundling PostService with database and site setup
    pub struct PostServiceTestContext {
        pub post_service: PostService,
        pub db_site: Arc<DbSite>,
        pub cache: Arc<WpApiCache>,
    }

    #[rstest]
    fn test_delete_by_entity_id(post_service_ctx: PostServiceTestContext) {
        // Setup: Insert test post
        let test_post = insert_test_post(&post_service_ctx);
        let entity_id = post_service_ctx
            .cache
            .execute(|conn| {
                let repo = PostRepository::<EditContext>::new();
                repo.select_by_post_id(conn, &post_service_ctx.db_site, test_post.id)
                    .map(|opt| opt.map(|full_entity| *full_entity.entity_id))
            })
            .expect("Database read should succeed")
            .expect("Post should exist");

        // Test: Delete by entity_id
        let deleted = post_service_ctx
            .post_service
            .delete_by_entity_id(&entity_id)
            .expect("Delete should succeed");

        // Assert: Post was deleted
        assert_eq!(deleted, 1, "Should delete 1 post");

        // Verify post no longer exists
        let result = post_service_ctx.cache.execute(|conn| {
            let repo = PostRepository::<EditContext>::new();
            repo.select_by_entity_id(conn, &entity_id)
        });
        assert!(
            result.unwrap().is_none(),
            "Post should not exist after deletion"
        );
    }

    #[rstest]
    fn test_delete_by_post_id(post_service_ctx: PostServiceTestContext) {
        // Setup: Insert test post
        let test_post = insert_test_post(&post_service_ctx);

        // Test: Delete by post_id
        let deleted = post_service_ctx
            .post_service
            .delete_by_post_id(test_post.id)
            .expect("Delete should succeed");

        // Assert: Post was deleted
        assert_eq!(deleted, 1, "Should delete 1 post");

        // Verify post no longer exists
        let result = post_service_ctx.cache.execute(|conn| {
            let repo = PostRepository::<EditContext>::new();
            repo.select_by_post_id(conn, &post_service_ctx.db_site, test_post.id)
        });
        assert!(
            result.unwrap().is_none(),
            "Post should not exist after deletion"
        );
    }

    #[rstest]
    fn test_delete_by_entity_id_non_existent_returns_zero(
        post_service_ctx: PostServiceTestContext,
    ) {
        // Setup: Insert a post and get its entity_id
        let test_post = insert_test_post(&post_service_ctx);
        let entity_id = post_service_ctx
            .cache
            .execute(|conn| {
                let repo = PostRepository::<EditContext>::new();
                repo.select_by_post_id(conn, &post_service_ctx.db_site, test_post.id)
                    .map(|opt| opt.map(|full_entity| *full_entity.entity_id))
            })
            .expect("Database read should succeed")
            .expect("Post should exist");

        // Setup: Delete the post via service
        post_service_ctx
            .post_service
            .delete_by_entity_id(&entity_id)
            .expect("First delete should succeed");

        // Test: Try to delete again with the same entity_id (now non-existent)
        let deleted = post_service_ctx
            .post_service
            .delete_by_entity_id(&entity_id)
            .expect("Delete should not error");

        // Assert: Should return 0
        assert_eq!(deleted, 0, "Should return 0 for non-existent post");
    }

    #[rstest]
    fn test_delete_by_post_id_non_existent_returns_zero(post_service_ctx: PostServiceTestContext) {
        // Test: Delete non-existent post
        let deleted = post_service_ctx
            .post_service
            .delete_by_post_id(PostId(99999))
            .expect("Delete should not error");

        // Assert: Should return 0
        assert_eq!(deleted, 0, "Should return 0 for non-existent post");
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
