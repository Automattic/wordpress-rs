use crate::{
    FullEntityPostTypeDetailsWithEditContext,
    collection::{CollectionError, FetchError, StatelessCollection},
    service::post_types::PostTypeService,
};
use std::sync::Arc;
use wp_mobile_cache::{
    UpdateHook,
    db_types::post_types::DbPostTypeDetailsWithEditContext,
    entity::{EntityId, FullEntity},
};

/// Stateless collection for post types with edit context.
///
/// Provides reactive access to cached post types without tracking state or pagination.
/// All post types are fetched in a single request (no pagination needed).
///
/// # Usage
///
/// ```ignore
/// use crate::filters::PostTypeFilter;
///
/// // Create filter to only show viewable post types
/// let filter = PostTypeFilter {
///     viewable: Some(true),
/// };
///
/// // Create collection
/// let collection = post_type_service.create_post_type_collection_with_edit_context(filter);
///
/// // Fetch all post types from API
/// collection.fetch().await?;
///
/// // Load cached post types (only viewable ones)
/// let post_types = collection.load_data().await?;
/// ```
#[derive(uniffi::Object)]
pub struct PostTypeCollectionWithEditContext {
    /// Underlying stateless collection for cache access
    stateless_collection: StatelessCollection<FullEntity<DbPostTypeDetailsWithEditContext>>,

    /// Reference to the service for network operations
    post_type_service: Arc<PostTypeService>,
}

impl PostTypeCollectionWithEditContext {
    /// Create a new post type collection
    pub fn new(
        stateless_collection: StatelessCollection<FullEntity<DbPostTypeDetailsWithEditContext>>,
        service: Arc<PostTypeService>,
    ) -> Self {
        Self {
            stateless_collection,
            post_type_service: service,
        }
    }
}

#[uniffi::export]
impl PostTypeCollectionWithEditContext {
    /// Fetch all post types from the network.
    ///
    /// This calls the network API and upserts all post types to the database.
    /// After successful fetch, the database change will trigger observers
    /// who can then call load_data() to get updated results.
    ///
    /// Unlike posts, post types don't support pagination - all types are
    /// returned in a single request.
    ///
    /// # Returns
    /// - `Ok(Vec<EntityId>)` with entity IDs of all synced post types
    /// - `Err(FetchError)` if network or database error occurs
    pub async fn fetch(&self) -> Result<Vec<EntityId>, FetchError> {
        self.post_type_service.sync_post_types().await
    }

    /// Load all cached post types from the database.
    ///
    /// This queries the database and returns all post types stored in the cache.
    /// It's an expensive operation that re-queries on every call (stateless behavior).
    ///
    /// # Returns
    /// - `Ok(Vec<FullEntity>>)` with all cached post types
    /// - `Err(CollectionError)` if database error occurs
    ///
    /// # Note
    /// This async function is exported to client platforms (Kotlin/Swift) where it
    /// will be executed on a background thread. The underlying Rust implementation
    /// is synchronous as rusqlite doesn't support async operations.
    pub async fn load_data(
        &self,
    ) -> Result<Vec<FullEntityPostTypeDetailsWithEditContext>, CollectionError> {
        self.stateless_collection
            .load_data()
            .map(|full_entities| {
                full_entities
                    .into_iter()
                    .map(|db_full_entity| {
                        // Convert FullEntity<DbPostTypeDetailsWithEditContext> to FullEntity<PostTypeDetailsWithEditContext>
                        let entity_id = db_full_entity.entity_id;
                        let post_type_details = db_full_entity.data.post_type;
                        FullEntity::new(entity_id, post_type_details).into()
                    })
                    .collect()
            })
            .map_err(|e| CollectionError::DatabaseError {
                err_message: e.to_string(),
            })
    }

    /// Check if a database update is relevant to this collection.
    ///
    /// Returns true if the update might affect post types in this collection.
    /// Used by platform-specific observable wrappers to determine whether to
    /// notify observers.
    pub fn is_relevant_update(&self, hook: &UpdateHook) -> bool {
        self.stateless_collection.is_relevant_update(hook)
    }
}
