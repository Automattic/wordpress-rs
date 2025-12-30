use crate::collection::{FetchError, PostTypeCollectionWithEditContext, StatelessCollection};
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

    /// Create a stateless collection for post types with edit context.
    ///
    /// The collection provides reactive access to cached post types with
    /// database change notifications.
    pub fn create_post_type_collection_with_edit_context(
        &self,
    ) -> Arc<PostTypeCollectionWithEditContext> {
        let cache = self.cache.clone();
        let db_site = self.db_site.clone();

        // Create the stateless collection with a closure that loads all post types
        let stateless_collection = StatelessCollection::new(
            vec![DbTable::PostTypesEditContext],
            Box::new(move || {
                cache.execute(|conn| {
                    let repo = PostTypeRepository::<EditContext>::new();
                    repo.select_all(conn, &db_site)
                })
            }),
        );

        Arc::new(PostTypeCollectionWithEditContext::new(
            stateless_collection,
            Arc::new(self.clone()),
        ))
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
