//! Post-specific implementation of `MetadataFetcher`.

use std::sync::Arc;

use wp_api::{posts::PostId, request::endpoint::posts_endpoint::PostEndpointType};

use crate::{
    collection::FetchError,
    filters::PostListFilter,
    service::posts::PostService,
    sync::{MetadataFetchResult, MetadataFetcher},
};

/// Database-backed `MetadataFetcher` implementation for posts with edit context.
///
/// Stores metadata to the persistent database via `MetadataService`, allowing
/// list metadata to survive app restarts.
///
/// # Usage
///
/// ```ignore
/// let fetcher = PersistentPostMetadataFetcherWithEditContext::new(
///     service.clone(),
///     PostEndpointType::Posts,
///     filter,
///     "site_1:edit:posts:status=publish".to_string(),
/// );
///
/// let mut collection = MetadataCollection::new(
///     "site_1:edit:posts:status=publish".to_string(),
///     service.persistent_metadata_reader(),  // DB-backed reader
///     service.state_reader_with_edit_context(),
///     fetcher,
///     vec![DbTable::PostsEditContext, DbTable::ListMetadataItems],
/// );
/// ```
pub struct PersistentPostMetadataFetcherWithEditContext {
    /// Reference to the post service
    service: Arc<PostService>,

    /// The post endpoint type (Posts, Pages, or Custom)
    endpoint_type: PostEndpointType,

    /// Filter parameters for the post list (excludes pagination)
    filter: PostListFilter,

    /// Key for metadata store lookup
    kv_key: String,
}

impl PersistentPostMetadataFetcherWithEditContext {
    /// Create a new persistent post metadata fetcher.
    ///
    /// # Arguments
    /// * `service` - The post service to delegate to
    /// * `endpoint_type` - The post endpoint type (Posts, Pages, or Custom)
    /// * `filter` - Filter parameters for the post list (pagination is managed internally)
    /// * `kv_key` - Key for the metadata store (e.g., "site_1:posts:status=publish")
    pub fn new(
        service: Arc<PostService>,
        endpoint_type: PostEndpointType,
        filter: PostListFilter,
        kv_key: String,
    ) -> Self {
        Self {
            service,
            endpoint_type,
            filter,
            kv_key,
        }
    }
}

impl MetadataFetcher for PersistentPostMetadataFetcherWithEditContext {
    async fn fetch_metadata(
        &self,
        page: u32,
        per_page: u32,
        is_first_page: bool,
    ) -> Result<MetadataFetchResult, FetchError> {
        self.service
            .fetch_and_store_metadata_persistent(
                &self.kv_key,
                &self.endpoint_type,
                &self.filter,
                page,
                per_page,
                is_first_page,
            )
            .await
    }

    async fn ensure_fetched(&self, ids: Vec<i64>) -> Result<(), FetchError> {
        let post_ids: Vec<PostId> = ids.into_iter().map(PostId).collect();
        self.service
            .fetch_posts_by_ids(&self.endpoint_type, post_ids)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // Integration tests for PostMetadataFetcherWithEditContext would require
    // a mock API client and database setup. These are better suited for
    // the integration test suite.
    //
    // Unit tests here would just verify construction, which is trivial.
}
