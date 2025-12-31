use crate::{
    collection::{FetchError, PostTypeCollectionWithEditContext, StatelessCollection},
    filters::PostTypeFilter,
};
use std::sync::Arc;
use wp_api::prelude::WpApiClient;
use wp_mobile_cache::{
    DbTable, WpApiCache,
    context::EditContext,
    db_types::db_site::DbSite,
    entity::EntityId,
    repository::post_types::PostTypeRepository,
};

/// Service layer for post type operations.
///
/// Provides a bridge between clients and the underlying network/cache layers.
/// Handles fetching and caching post types for a site.
#[derive(uniffi::Object)]
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

        // Upsert each post type to the database
        let entity_ids = self
            .cache
            .execute(|conn| {
                let repo = PostTypeRepository::<EditContext>::new();
                let mut ids = Vec::new();

                for (post_type_enum, post_type_details) in post_types_map.iter() {
                    let slug = post_type_enum.to_string();
                    let entity_id = repo.upsert(conn, &self.db_site, &slug, post_type_details)?;
                    ids.push(entity_id);
                }

                Ok::<Vec<EntityId>, wp_mobile_cache::SqliteDbError>(ids)
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
                            if let Some(viewable_value) = filter_clone.viewable {
                                if entity.data.post_type.viewable != viewable_value {
                                    return false;
                                }
                            }

                            // Filter by show_ui
                            if let Some(show_ui_value) = filter_clone.show_ui {
                                if entity.data.post_type.visibility.show_ui != show_ui_value {
                                    return false;
                                }
                            }

                            // Filter by hierarchical
                            if let Some(hierarchical_value) = filter_clone.hierarchical {
                                if entity.data.post_type.hierarchical != hierarchical_value {
                                    return false;
                                }
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

// Implement Clone manually since we need it for the collection
impl Clone for PostTypeService {
    fn clone(&self) -> Self {
        Self {
            db_site: self.db_site.clone(),
            api_client: self.api_client.clone(),
            cache: self.cache.clone(),
        }
    }
}
