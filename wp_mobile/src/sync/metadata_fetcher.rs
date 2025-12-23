use crate::collection::FetchError;

use super::SyncResult;

/// Trait for syncing entity lists.
///
/// Implementations handle the full sync flow:
/// 1. Fetching lightweight metadata (id + modified_gmt) for list structure
/// 2. Storing metadata in the database
/// 3. Fetching full entities that are missing or stale in the cache
///
/// The service layer provides concrete implementations that know how to
/// fetch specific entity types (posts, media, etc.) and update the
/// appropriate stores.
///
/// # Example Implementation
///
/// ```ignore
/// struct PostMetadataFetcher {
///     service: Arc<PostService>,
///     endpoint_type: PostEndpointType,
///     filter: PostListFilter,
///     key: ListKey,
/// }
///
/// impl MetadataFetcher for PostMetadataFetcher {
///     async fn sync(&self, per_page: u32, is_refresh: bool) -> Result<SyncResult, FetchError> {
///         self.service.sync_list(
///             &self.key,
///             &self.endpoint_type,
///             &self.filter,
///             per_page,
///             is_refresh,
///         ).await
///     }
/// }
/// ```
pub trait MetadataFetcher: Send + Sync {
    /// Sync a list: fetch metadata and missing/stale entities.
    ///
    /// This performs the full sync flow:
    /// 1. Fetch list metadata (IDs, modified_gmt, pagination)
    /// 2. Store metadata in the database
    /// 3. Detect and mark stale entities
    /// 4. Fetch missing/stale entities from the API
    ///
    /// # Arguments
    /// * `per_page` - Number of items per page
    /// * `is_refresh` - If true, refreshes (page 1); if false, loads more (next page)
    ///
    /// # Returns
    /// Sync statistics including counts and pagination info.
    fn sync(
        &self,
        per_page: u32,
        is_refresh: bool,
    ) -> impl std::future::Future<Output = Result<SyncResult, FetchError>> + Send;
}
