//! Post-specific implementation of `MetadataFetcher`.

use std::sync::Arc;

use wp_api::posts::{PostId, PostListParams};

use crate::{
    collection::FetchError,
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
///     params,
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

    /// API parameters for the post list
    params: PostListParams,

    /// Key for metadata store lookup
    kv_key: String,
}

impl PersistentPostMetadataFetcherWithEditContext {
    /// Create a new persistent post metadata fetcher.
    ///
    /// # Arguments
    /// * `service` - The post service to delegate to
    /// * `params` - API parameters for the post list query
    /// * `kv_key` - Key for the metadata store (e.g., "site_1:posts:status=publish")
    pub fn new(service: Arc<PostService>, params: PostListParams, kv_key: String) -> Self {
        Self {
            service,
            params,
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
                &self.params,
                page,
                per_page,
                is_first_page,
            )
            .await
    }

    async fn ensure_fetched(&self, ids: Vec<i64>) -> Result<(), FetchError> {
        let post_ids: Vec<PostId> = ids.into_iter().map(PostId).collect();
        self.service.fetch_posts_by_ids(post_ids).await?;
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
