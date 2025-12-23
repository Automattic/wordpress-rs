use crate::collection::FetchError;

use super::MetadataFetchResult;

/// Trait for fetching entity metadata and full entities.
///
/// Implementations of this trait handle:
/// 1. Fetching lightweight metadata (id + modified_gmt) for list structure
/// 2. Fetching full entities by ID and storing them in the cache
///
/// The service layer provides concrete implementations that know how to
/// fetch specific entity types (posts, media, etc.) and update the
/// appropriate stores.
///
/// # Page Number Handling
///
/// The `page` parameter is no longer passed from `MetadataCollection` to `fetch_metadata`.
/// Instead, `MetadataService` internally determines the correct page:
/// - `refresh` always fetches page 1
/// - `load_more` determines the next page from persisted state
///
/// # Example Implementation
///
/// ```ignore
/// struct PostMetadataFetcher<'a> {
///     service: &'a PostServiceWithEditContext,
///     filter: AnyPostFilter,
///     kv_key: String,
/// }
///
/// impl MetadataFetcher for PostMetadataFetcher<'_> {
///     async fn fetch_metadata(&self, per_page: u32, is_first_page: bool)
///         -> Result<MetadataFetchResult, FetchError>
///     {
///         self.service.fetch_and_store_metadata(
///             &self.kv_key, &self.filter, per_page, is_first_page
///         ).await
///     }
///
///     async fn ensure_fetched(&self, ids: Vec<i64>) -> Result<(), FetchError> {
///         let post_ids = ids.into_iter().map(PostId).collect();
///         self.service.fetch_posts_by_ids(post_ids).await
///     }
/// }
/// ```
pub trait MetadataFetcher: Send + Sync {
    /// Fetch metadata for a page and store in the metadata store.
    ///
    /// # Arguments
    /// * `per_page` - Number of items per page
    /// * `is_first_page` - If true, refreshes (page 1); if false, loads more (next page)
    ///
    /// # Returns
    /// Metadata for the fetched page, including pagination info.
    ///
    /// # Note
    /// The page number is determined internally by `MetadataService`:
    /// - `refresh` always fetches page 1
    /// - `load_more` determines the next page from persisted state
    fn fetch_metadata(
        &self,
        per_page: u32,
        is_first_page: bool,
    ) -> impl std::future::Future<Output = Result<MetadataFetchResult, FetchError>> + Send;

    /// Ensure entities are fetched and cached.
    ///
    /// This fetches full entity data for the given IDs and stores them
    /// in the database cache. It also updates the entity state store
    /// (Fetching → Cached/Failed).
    ///
    /// # Arguments
    /// * `ids` - Entity IDs to fetch (as raw i64 values)
    fn ensure_fetched(
        &self,
        ids: Vec<i64>,
    ) -> impl std::future::Future<Output = Result<(), FetchError>> + Send;
}
