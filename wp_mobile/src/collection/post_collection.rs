use crate::{AnyPostFilter, FetchError, FetchResult, NaiveCollection, service::posts::PostService};
use std::sync::Arc;
use wp_mobile_cache::entity::FullEntity;

/// Collection of posts with context-specific data
///
/// Wraps NaiveCollection and adds:
/// - Type-specific filter configuration
/// - Network fetching via service layer
///
/// Generic over the post type (AnyPostWithEditContext, AnyPostWithViewContext, etc.)
pub struct PostCollection<T> {
    /// The filter defining which posts belong to this collection
    filter: AnyPostFilter,

    /// Underlying naive collection for cache access
    naive_collection: NaiveCollection<FullEntity<T>>,

    /// Reference to the service for network operations
    post_service: Arc<PostService>,
}

impl<T> PostCollection<T> {
    /// Create a new post collection
    pub fn new(
        filter: AnyPostFilter,
        naive_collection: NaiveCollection<FullEntity<T>>,
        service: Arc<PostService>,
    ) -> Self {
        Self {
            filter,
            naive_collection,
            post_service: service,
        }
    }

    /// Get the filter for this collection
    pub fn filter(&self) -> &AnyPostFilter {
        &self.filter
    }

    /// Fetch a specific page from the network
    ///
    /// This calls the network API and upserts results to the database.
    /// After successful fetch, the database change will trigger observers
    /// who can then call load_data() to get updated results.
    ///
    /// # Arguments
    /// * `page` - Page number to fetch (1-indexed)
    /// * `per_page` - Number of posts per page
    ///
    /// # Returns
    /// - `Ok(FetchResult)` with entity IDs and pagination info
    /// - `Err(FetchError)` if network or database error occurs
    ///
    /// # Note
    /// This is a stateless operation - the collection doesn't track
    /// which pages have been fetched. ViewModels manage pagination state.
    pub async fn fetch_page(&self, page: u32, per_page: u32) -> Result<FetchResult, FetchError> {
        self.post_service
            .fetch_posts_page(&self.filter, page, per_page)
            .await
    }

    /// Load all cached items matching this collection's filter
    ///
    /// This queries the database and returns all posts that match
    /// the collection's filter criteria. It's an expensive operation
    /// that re-queries on every call (naive behavior).
    ///
    /// # Returns
    /// - `Ok(Vec<FullEntity>>)` with all matching posts from cache
    /// - `Err(CollectionError)` if database error occurs
    pub fn load_data(&self) -> Result<Vec<FullEntity<T>>, crate::CollectionError> {
        self.naive_collection
            .load_data()
            .map_err(|e| crate::CollectionError::DatabaseError {
                err_message: e.to_string(),
            })
    }

    /// Check if a database update is relevant to this collection
    ///
    /// Returns true if the update might affect posts in this collection.
    /// Used by platform-specific observable wrappers to determine
    /// whether to notify observers.
    pub fn is_relevant_update(&self, hook: &wp_mobile_cache::UpdateHook) -> bool {
        self.naive_collection.is_relevant_update(hook)
    }
}
