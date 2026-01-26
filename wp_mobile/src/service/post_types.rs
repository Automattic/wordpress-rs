use crate::{
    collection::{FetchError, PostTypeCollectionWithEditContext, StatelessCollection},
    filters::PostTypeFilter,
};
use std::sync::Arc;
use wp_api::prelude::WpApiClient;
use wp_mobile_cache::{
    DbTable, WpApiCache, context::EditContext, db_types::db_site::DbSite, entity::EntityId,
    repository::post_types::PostTypeRepository,
};

/// Service layer for post type operations.
///
/// Provides a bridge between clients and the underlying network/cache layers.
/// Handles fetching and caching post types for a site.
#[derive(Clone, uniffi::Object)]
pub struct PostTypeService {
    db_site: Arc<DbSite>,
    api_client: Arc<WpApiClient>,
    cache: Arc<WpApiCache>,
}

impl PostTypeService {
    pub fn new(api_client: Arc<WpApiClient>, db_site: Arc<DbSite>, cache: Arc<WpApiCache>) -> Self {
        Self {
            api_client,
            db_site,
            cache,
        }
    }
}

#[uniffi::export]
impl PostTypeService {
    /// Sync all post types from the API and cache them.
    ///
    /// Fetches the HashMap of post types from the WordPress API and upserts
    /// each one to the database.
    ///
    /// # Returns
    /// Vector of EntityIds for all synced post types
    pub async fn sync_post_types(&self) -> Result<Vec<EntityId>, FetchError> {
        // Fetch post types from API
        let response = self
            .api_client
            .post_types()
            .list_with_edit_context()
            .await?;

        // Extract the HashMap from the response
        let post_types_map = response.data.post_types;

        // Delete all existing post types and insert new ones
        let entity_ids = self
            .cache
            .execute(|conn| {
                let repo = PostTypeRepository::<EditContext>::new();

                repo.delete_all(conn, &self.db_site)?;

                post_types_map
                    .iter()
                    .map(|(post_type_enum, post_type_details)| {
                        let slug = post_type_enum.to_string();
                        repo.upsert(conn, &self.db_site, &slug, post_type_details)
                    })
                    .collect::<Result<Vec<EntityId>, wp_mobile_cache::SqliteDbError>>()
            })
            .map_err(|e| FetchError::Database {
                err_message: e.to_string(),
            })?;

        Ok(entity_ids)
    }

    /// Create a stateless collection for post types with edit context using default filter.
    ///
    /// The collection provides reactive access to cached post types with
    /// database change notifications.
    ///
    /// By default, only viewable and UI-visible post types are returned
    /// (those with `viewable = true` and `show_ui = true`).
    /// For custom filtering, use `create_post_type_collection_with_edit_context_filtered()`.
    ///
    /// # Example (Kotlin)
    /// ```kotlin
    /// // Get viewable and UI-visible post types (default behavior)
    /// val collection = postTypeService.createPostTypeCollectionWithEditContext()
    /// ```
    pub fn create_post_type_collection_with_edit_context(
        &self,
    ) -> Arc<PostTypeCollectionWithEditContext> {
        self.create_post_type_collection_with_edit_context_filtered(PostTypeFilter::default())
    }

    /// Create a stateless collection for post types with edit context using a custom filter.
    ///
    /// The collection provides reactive access to cached post types with
    /// database change notifications.
    ///
    /// # Arguments
    /// * `filter` - Filter criteria for post types (viewable, show_ui, hierarchical)
    ///
    /// # Example (Kotlin)
    /// ```kotlin
    /// // Get all post types (no filtering)
    /// val allFilter = PostTypeFilter(viewable = null, showUi = null, hierarchical = null)
    /// val allCollection = postTypeService.createPostTypeCollectionWithEditContextFiltered(allFilter)
    ///
    /// // Get only hierarchical post types that are viewable and shown in UI
    /// val hierarchicalFilter = PostTypeFilter(viewable = true, showUi = true, hierarchical = true)
    /// val hierarchicalCollection = postTypeService.createPostTypeCollectionWithEditContextFiltered(hierarchicalFilter)
    ///
    /// // Get hidden/internal post types
    /// val hiddenFilter = PostTypeFilter(viewable = null, showUi = false, hierarchical = null)
    /// val hiddenCollection = postTypeService.createPostTypeCollectionWithEditContextFiltered(hiddenFilter)
    /// ```
    pub fn create_post_type_collection_with_edit_context_filtered(
        &self,
        filter: PostTypeFilter,
    ) -> Arc<PostTypeCollectionWithEditContext> {
        let cache = self.cache.clone();
        let db_site = self.db_site.clone();
        let filter_clone = filter.clone();

        // Create the stateless collection with a closure that loads and filters post types
        let stateless_collection = StatelessCollection::new(
            vec![DbTable::PostTypesEditContext],
            Box::new(move || {
                cache.execute(|conn| {
                    let repo = PostTypeRepository::<EditContext>::new();
                    let all_post_types = repo.select_all(conn, &db_site)?;

                    // Apply filtering in memory
                    let filtered = all_post_types
                        .into_iter()
                        .filter(|entity| {
                            // Filter by viewable
                            if let Some(viewable_value) = filter_clone.viewable
                                && entity.data.post_type.viewable != viewable_value
                            {
                                return false;
                            }

                            // Filter by show_ui
                            if let Some(show_ui_value) = filter_clone.show_ui
                                && entity.data.post_type.visibility.show_ui != show_ui_value
                            {
                                return false;
                            }

                            // Filter by hierarchical
                            if let Some(hierarchical_value) = filter_clone.hierarchical
                                && entity.data.post_type.hierarchical != hierarchical_value
                            {
                                return false;
                            }

                            true
                        })
                        .collect();

                    Ok(filtered)
                })
            }),
        );

        Arc::new(PostTypeCollectionWithEditContext::new(
            stateless_collection,
            Arc::new(self.clone()),
        ))
    }

    /// Get a post type by slug from the cache.
    ///
    /// # Returns
    /// The post type details with edit context if found, None otherwise
    pub fn get_by_slug(
        &self,
        slug: String,
    ) -> Option<wp_api::post_types::PostTypeDetailsWithEditContext> {
        self.cache
            .execute(|conn| {
                let repo = PostTypeRepository::<EditContext>::new();
                repo.select_by_slug(conn, &self.db_site, &slug)
            })
            .ok()
            .flatten()
            .map(|db_full_entity| db_full_entity.data.post_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::mock_api_client;
    use futures::executor::block_on;
    use rstest::*;
    use rusqlite::Connection;
    use wp_mobile_cache::{
        MigrationManager, WpApiCache,
        db_types::self_hosted_site::SelfHostedSite,
        test_fixtures::{create_test_site, post_types::PostTypeBuilder},
    };

    struct PostTypeServiceTestContext {
        cache: Arc<WpApiCache>,
        db_site: Arc<DbSite>,
        post_type_service: PostTypeService,
    }

    #[fixture]
    fn post_type_service_ctx() -> PostTypeServiceTestContext {
        let mut conn = Connection::open_in_memory().unwrap();
        let mut migration_manager = MigrationManager::new(&conn).unwrap();
        migration_manager
            .perform_migrations()
            .expect("Migrations should succeed");

        let self_hosted_site = SelfHostedSite {
            url: "https://test.local".to_string(),
            api_root: "https://test.local/wp-json".to_string(),
        };
        let db_site = create_test_site(&mut conn, &self_hosted_site);

        let cache = Arc::new(WpApiCache::from(conn));
        let api_client = mock_api_client();

        let post_type_service = PostTypeService::new(api_client, Arc::new(db_site), cache.clone());

        PostTypeServiceTestContext {
            cache,
            db_site: Arc::new(db_site),
            post_type_service,
        }
    }

    /// Helper to insert a post type into the test database
    fn insert_post_type(
        ctx: &PostTypeServiceTestContext,
        post_type: &wp_api::post_types::PostTypeDetailsWithEditContext,
    ) {
        ctx.cache
            .execute(|conn| {
                let repo = PostTypeRepository::<EditContext>::new();
                repo.upsert(conn, &ctx.db_site, &post_type.slug, post_type)
            })
            .expect("Failed to insert post type");
    }

    #[rstest]
    fn test_filter_default_returns_only_viewable_and_shown_types(
        post_type_service_ctx: PostTypeServiceTestContext,
    ) {
        // Insert 4 post types with different visibility combinations
        let visible = PostTypeBuilder::new("post")
            .viewable(true)
            .show_ui(true)
            .build();
        let hidden_ui = PostTypeBuilder::new("revision")
            .viewable(true)
            .show_ui(false)
            .build();
        let not_viewable = PostTypeBuilder::new("nav_menu_item")
            .viewable(false)
            .show_ui(true)
            .build();
        let completely_hidden = PostTypeBuilder::new("custom_css")
            .viewable(false)
            .show_ui(false)
            .build();

        insert_post_type(&post_type_service_ctx, &visible);
        insert_post_type(&post_type_service_ctx, &hidden_ui);
        insert_post_type(&post_type_service_ctx, &not_viewable);
        insert_post_type(&post_type_service_ctx, &completely_hidden);

        // Use default filter (viewable=true, show_ui=true)
        let collection = post_type_service_ctx
            .post_type_service
            .create_post_type_collection_with_edit_context();

        let result = block_on(collection.load_data()).expect("load_data should succeed");

        // Should only return the one that's both viewable AND show_ui
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].data.slug, "post");
    }

    #[rstest]
    fn test_filter_by_viewable_only(post_type_service_ctx: PostTypeServiceTestContext) {
        // Insert viewable and non-viewable types
        let viewable1 = PostTypeBuilder::new("post")
            .viewable(true)
            .show_ui(true)
            .build();
        let viewable2 = PostTypeBuilder::new("page")
            .viewable(true)
            .show_ui(false)
            .build();
        let not_viewable = PostTypeBuilder::new("revision")
            .viewable(false)
            .show_ui(true)
            .build();

        insert_post_type(&post_type_service_ctx, &viewable1);
        insert_post_type(&post_type_service_ctx, &viewable2);
        insert_post_type(&post_type_service_ctx, &not_viewable);

        // Filter only by viewable=true, ignore show_ui
        let filter = PostTypeFilter {
            viewable: Some(true),
            show_ui: None,
            hierarchical: None,
        };

        let collection = post_type_service_ctx
            .post_type_service
            .create_post_type_collection_with_edit_context_filtered(filter);

        let result = block_on(collection.load_data()).expect("load_data should succeed");

        // Should return both viewable types regardless of show_ui
        assert_eq!(result.len(), 2);
        let slugs: Vec<_> = result.iter().map(|e| e.data.slug.as_str()).collect();
        assert!(slugs.contains(&"post"));
        assert!(slugs.contains(&"page"));
    }

    #[rstest]
    fn test_filter_by_show_ui_only(post_type_service_ctx: PostTypeServiceTestContext) {
        // Insert types with different show_ui values
        let shown1 = PostTypeBuilder::new("post")
            .viewable(true)
            .show_ui(true)
            .build();
        let shown2 = PostTypeBuilder::new("attachment")
            .viewable(false)
            .show_ui(true)
            .build();
        let hidden = PostTypeBuilder::new("revision")
            .viewable(true)
            .show_ui(false)
            .build();

        insert_post_type(&post_type_service_ctx, &shown1);
        insert_post_type(&post_type_service_ctx, &shown2);
        insert_post_type(&post_type_service_ctx, &hidden);

        // Filter only by show_ui=true, ignore viewable
        let filter = PostTypeFilter {
            viewable: None,
            show_ui: Some(true),
            hierarchical: None,
        };

        let collection = post_type_service_ctx
            .post_type_service
            .create_post_type_collection_with_edit_context_filtered(filter);

        let result = block_on(collection.load_data()).expect("load_data should succeed");

        // Should return both show_ui=true types regardless of viewable
        assert_eq!(result.len(), 2);
        let slugs: Vec<_> = result.iter().map(|e| e.data.slug.as_str()).collect();
        assert!(slugs.contains(&"post"));
        assert!(slugs.contains(&"attachment"));
    }

    #[rstest]
    fn test_filter_by_hierarchical_only(post_type_service_ctx: PostTypeServiceTestContext) {
        // Insert hierarchical and flat types
        let hierarchical1 = PostTypeBuilder::new("page")
            .viewable(true)
            .show_ui(true)
            .hierarchical(true)
            .build();
        let hierarchical2 = PostTypeBuilder::new("custom_hierarchical")
            .viewable(false)
            .show_ui(false)
            .hierarchical(true)
            .build();
        let flat = PostTypeBuilder::new("post")
            .viewable(true)
            .show_ui(true)
            .hierarchical(false)
            .build();

        insert_post_type(&post_type_service_ctx, &hierarchical1);
        insert_post_type(&post_type_service_ctx, &hierarchical2);
        insert_post_type(&post_type_service_ctx, &flat);

        // Filter only by hierarchical=true
        let filter = PostTypeFilter {
            viewable: None,
            show_ui: None,
            hierarchical: Some(true),
        };

        let collection = post_type_service_ctx
            .post_type_service
            .create_post_type_collection_with_edit_context_filtered(filter);

        let result = block_on(collection.load_data()).expect("load_data should succeed");

        // Should return both hierarchical types
        assert_eq!(result.len(), 2);
        let slugs: Vec<_> = result.iter().map(|e| e.data.slug.as_str()).collect();
        assert!(slugs.contains(&"page"));
        assert!(slugs.contains(&"custom_hierarchical"));
    }

    #[rstest]
    fn test_filter_all_criteria_combined(post_type_service_ctx: PostTypeServiceTestContext) {
        // Insert types with all combinations
        let matches_all = PostTypeBuilder::new("page")
            .viewable(true)
            .show_ui(true)
            .hierarchical(true)
            .build();
        let wrong_viewable = PostTypeBuilder::new("type1")
            .viewable(false)
            .show_ui(true)
            .hierarchical(true)
            .build();
        let wrong_show_ui = PostTypeBuilder::new("type2")
            .viewable(true)
            .show_ui(false)
            .hierarchical(true)
            .build();
        let wrong_hierarchical = PostTypeBuilder::new("post")
            .viewable(true)
            .show_ui(true)
            .hierarchical(false)
            .build();

        insert_post_type(&post_type_service_ctx, &matches_all);
        insert_post_type(&post_type_service_ctx, &wrong_viewable);
        insert_post_type(&post_type_service_ctx, &wrong_show_ui);
        insert_post_type(&post_type_service_ctx, &wrong_hierarchical);

        // Filter by all three criteria
        let filter = PostTypeFilter {
            viewable: Some(true),
            show_ui: Some(true),
            hierarchical: Some(true),
        };

        let collection = post_type_service_ctx
            .post_type_service
            .create_post_type_collection_with_edit_context_filtered(filter);

        let result = block_on(collection.load_data()).expect("load_data should succeed");

        // Should only return the one matching ALL criteria
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].data.slug, "page");
    }

    #[rstest]
    fn test_filter_with_all_none_returns_everything(
        post_type_service_ctx: PostTypeServiceTestContext,
    ) {
        // Insert various types
        let type1 = PostTypeBuilder::new("post")
            .viewable(true)
            .show_ui(true)
            .hierarchical(false)
            .build();
        let type2 = PostTypeBuilder::new("page")
            .viewable(true)
            .show_ui(false)
            .hierarchical(true)
            .build();
        let type3 = PostTypeBuilder::new("revision")
            .viewable(false)
            .show_ui(false)
            .hierarchical(false)
            .build();

        insert_post_type(&post_type_service_ctx, &type1);
        insert_post_type(&post_type_service_ctx, &type2);
        insert_post_type(&post_type_service_ctx, &type3);

        // Filter with all None (no filtering)
        let filter = PostTypeFilter {
            viewable: None,
            show_ui: None,
            hierarchical: None,
        };

        let collection = post_type_service_ctx
            .post_type_service
            .create_post_type_collection_with_edit_context_filtered(filter);

        let result = block_on(collection.load_data()).expect("load_data should succeed");

        // Should return all types
        assert_eq!(result.len(), 3);
    }

    #[rstest]
    fn test_filter_by_false_values(post_type_service_ctx: PostTypeServiceTestContext) {
        // Insert types to test filtering by false values
        let hidden_type = PostTypeBuilder::new("revision")
            .viewable(false)
            .show_ui(false)
            .hierarchical(false)
            .build();
        let visible_type = PostTypeBuilder::new("post")
            .viewable(true)
            .show_ui(true)
            .hierarchical(true)
            .build();

        insert_post_type(&post_type_service_ctx, &hidden_type);
        insert_post_type(&post_type_service_ctx, &visible_type);

        // Filter for non-viewable types
        let filter = PostTypeFilter {
            viewable: Some(false),
            show_ui: None,
            hierarchical: None,
        };

        let collection = post_type_service_ctx
            .post_type_service
            .create_post_type_collection_with_edit_context_filtered(filter);

        let result = block_on(collection.load_data()).expect("load_data should succeed");

        // Should only return the non-viewable type
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].data.slug, "revision");
        assert!(!result[0].data.viewable);
    }
}
