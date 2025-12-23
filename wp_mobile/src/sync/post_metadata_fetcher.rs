//! Post-specific implementation of `MetadataFetcher`.

use std::sync::Arc;

use wp_api::request::endpoint::posts_endpoint::PostEndpointType;
use wp_mobile_cache::list_metadata::ListKey;

use crate::{
    collection::FetchError,
    filters::PostListFilter,
    service::posts::PostService,
    sync::{MetadataFetcher, SyncResult},
};

/// Database-backed `MetadataFetcher` implementation for posts with edit context.
///
/// Delegates to `PostService::sync_list` which handles:
/// 1. Fetching and storing list metadata
/// 2. Detecting stale posts
/// 3. Fetching missing/stale posts
///
/// # Usage
///
/// ```ignore
/// let fetcher = PersistentPostMetadataFetcherWithEditContext::new(
///     service.clone(),
///     PostEndpointType::Posts,
///     filter,
///     ListKey::from("site_1:edit:posts:status=publish"),
/// );
///
/// let mut collection = MetadataCollection::new(
///     ListKey::from("site_1:edit:posts:status=publish"),
///     service.persistent_metadata_reader(),
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
    key: ListKey,
}

impl PersistentPostMetadataFetcherWithEditContext {
    /// Create a new persistent post metadata fetcher.
    ///
    /// # Arguments
    /// * `service` - The post service to delegate to
    /// * `endpoint_type` - The post endpoint type (Posts, Pages, or Custom)
    /// * `filter` - Filter parameters for the post list (pagination is managed internally)
    /// * `key` - Key for the metadata store (e.g., "site_1:posts:status=publish")
    pub fn new(
        service: Arc<PostService>,
        endpoint_type: PostEndpointType,
        filter: PostListFilter,
        key: ListKey,
    ) -> Self {
        Self {
            service,
            endpoint_type,
            filter,
            key,
        }
    }
}

impl MetadataFetcher for PersistentPostMetadataFetcherWithEditContext {
    async fn sync(&self, per_page: u32, is_refresh: bool) -> Result<SyncResult, FetchError> {
        self.service
            .sync_list(
                &self.key,
                &self.endpoint_type,
                &self.filter,
                per_page,
                is_refresh,
            )
            .await
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
