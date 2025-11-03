use crate::{EntityAnyPostWithEditContext, entity::Entity};
use std::sync::Arc;
use wp_api::{
    api_client::WpApiClient,
    posts::{AnyPostWithEditContext, PostId},
};
use wp_mobile_cache::{WpApiCache, db_types::db_site::DbSite};

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
    /// Get an entity handle for a specific post
    ///
    /// Returns an entity that can be used to read post data and observe changes.
    /// The entity is lightweight - it doesn't fetch data until you call data() on it.
    pub fn get_entity(&self, id: PostId) -> EntityAnyPostWithEditContext {
        Entity::<AnyPostWithEditContext>::new(id.0, self.db_site.clone()).into()
    }
}
