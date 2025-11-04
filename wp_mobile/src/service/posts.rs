use crate::{EntityAnyPostWithEditContext, entity::Entity, entity_error::EntityError};
use std::sync::Arc;
use wp_api::{
    api_client::WpApiClient,
    posts::{AnyPostWithEditContext, PostId},
};
use wp_mobile_cache::{
    WpApiCache,
    db_types::db_site::DbSite,
    repository::posts::PostRepository,
    context::EditContext,
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
    fn read_post_from_db(&self, id: PostId) -> Result<Option<AnyPostWithEditContext>, EntityError> {
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
    /// Returns an entity that can be used to read post data and observe changes.
    /// The entity is lightweight - it doesn't fetch data until you call data() on it.
    pub fn get_entity(&self, id: PostId) -> EntityAnyPostWithEditContext {
        let cache = self.cache.clone();
        let db_site = self.db_site.clone();
        let id_val = id.0;

        Entity::<AnyPostWithEditContext>::new(
            id.0,
            Box::new(move || {
                Self::read_post_from_db_internal(&cache, &db_site, PostId(id_val))
            })
        ).into()
    }
}
