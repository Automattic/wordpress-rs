//! Post-specific implementation of `MetadataFetcher`.

use std::sync::Arc;

use wp_api::posts::PostId;

use crate::{
    collection::FetchError,
    filters::AnyPostFilter,
    service::posts::PostService,
    sync::{MetadataFetchResult, MetadataFetcher},
};

/// `MetadataFetcher` implementation for posts with edit context.
///
/// This fetcher delegates to `PostService` methods:
/// - `fetch_metadata` → `PostService::fetch_and_store_metadata`
/// - `ensure_fetched` → `PostService::fetch_posts_by_ids`
///
/// # Usage
///
/// ```ignore
/// let fetcher = PostMetadataFetcherWithEditContext::new(
///     service.clone(),
///     filter,
///     "site_1:edit:posts:publish".to_string(),
/// );
///
/// let mut collection = MetadataCollection::new(
///     "site_1:edit:posts:publish".to_string(),
///     service.metadata_reader(),
///     service.state_reader_with_edit_context(),
///     fetcher,
///     vec![DbTable::PostsEditContext],
/// );
/// ```
pub struct PostMetadataFetcherWithEditContext {
    /// Reference to the post service
    service: Arc<PostService>,

    /// Filter for the post list
    filter: AnyPostFilter,

    /// Key for metadata store lookup
    kv_key: String,
}

impl PostMetadataFetcherWithEditContext {
    /// Create a new post metadata fetcher.
    ///
    /// # Arguments
    /// * `service` - The post service to delegate to
    /// * `filter` - Filter criteria for the post list
    /// * `kv_key` - Key for the metadata store (e.g., "site_1:posts:publish")
    pub fn new(service: Arc<PostService>, filter: AnyPostFilter, kv_key: String) -> Self {
        Self {
            service,
            filter,
            kv_key,
        }
    }
}

impl MetadataFetcher for PostMetadataFetcherWithEditContext {
    async fn fetch_metadata(
        &self,
        page: u32,
        per_page: u32,
        is_first_page: bool,
    ) -> Result<MetadataFetchResult, FetchError> {
        self.service
            .fetch_and_store_metadata(&self.kv_key, &self.filter, page, per_page, is_first_page)
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
