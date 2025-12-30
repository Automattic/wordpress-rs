//! Post-specific metadata collection for efficient list syncing.

use std::sync::Arc;

use wp_api::posts::AnyPostWithEditContext;
use wp_api::request::endpoint::posts_endpoint::PostEndpointType;
use wp_mobile_cache::{UpdateHook, entity::FullEntity};

use crate::{
    collection::{CollectionError, FetchError},
    filters::PostListFilter,
    service::posts::PostService,
    sync::{EntityState, ListInfo, MetadataCollectionCore, SyncResult},
};

// Generate PostItemState enum and PostMetadataCollectionItem struct using the macro
crate::wp_mobile_metadata_item!(
    PostMetadataCollectionItem,
    PostItemState,
    crate::FullEntityAnyPostWithEditContext
);

/// Metadata-first collection for posts with edit context.
///
/// This collection uses a two-phase sync strategy:
/// 1. Fetch lightweight metadata (id + modified_gmt) to define list structure
/// 2. Selectively fetch full data for missing or stale items
///
/// Unlike `PostCollectionWithEditContext` which fetches full data for all items,
/// this collection shows cached items immediately and fetches only what's needed.
///
/// # Usage
///
/// ```ignore
/// // Create collection
/// let collection = post_service.create_post_metadata_collection_with_edit_context(
///     endpoint_type,
///     filter,
///     20, // per_page
/// );
///
/// // Initial load - fetches metadata, then syncs missing items
/// collection.refresh().await?;
///
/// // Get items with states and data
/// let items = collection.load_items()?;
/// for item in items {
///     match item.state {
///         PostItemState::Cached { data } => { /* show data */ }
///         PostItemState::Stale { data } => { /* show data, maybe refresh */ }
///         PostItemState::FetchingWithData { data } => { /* show data + loading */ }
///         PostItemState::FailedWithData { error, data } => { /* show data + error */ }
///         PostItemState::Fetching => { /* show loading placeholder */ }
///         PostItemState::Missing => { /* show placeholder */ }
///         PostItemState::Failed { error } => { /* show error */ }
///     }
/// }
///
/// // Load more
/// collection.load_next_page().await?;
/// ```
#[derive(uniffi::Object)]
pub struct PostMetadataCollectionWithEditContext {
    /// Core collection infrastructure (shared query logic)
    core: MetadataCollectionCore,

    /// Reference to service for sync operations and loading entity data
    service: Arc<PostService>,

    /// The post endpoint type (Posts, Pages, or Custom)
    endpoint_type: PostEndpointType,

    /// Filter parameters for the post list
    filter: PostListFilter,
}

impl PostMetadataCollectionWithEditContext {
    pub fn new(
        core: MetadataCollectionCore,
        service: Arc<PostService>,
        endpoint_type: PostEndpointType,
        filter: PostListFilter,
    ) -> Self {
        Self {
            core,
            service,
            endpoint_type,
            filter,
        }
    }
}

#[uniffi::export]
impl PostMetadataCollectionWithEditContext {
    /// Load all items with their current states and data.
    ///
    /// Returns items in list order with type-safe state representation.
    /// Each item's `state` is a [`PostItemState`] variant that encodes both
    /// the sync status and data availability.
    ///
    /// This is the primary method for getting collection contents to display.
    ///
    /// # Note
    /// Data availability is independent of the internal `EntityState`. After an app
    /// restart, items may have internal state `Missing` but still have cached data
    /// available. This method will return `FetchingWithData`, `Stale`, or `FailedWithData`
    /// variants appropriately when cached data exists.
    ///
    /// This async function is exported to client platforms (Kotlin/Swift) where it
    /// will be executed on a background thread. The underlying Rust implementation
    /// is synchronous as rusqlite doesn't support async operations.
    pub async fn load_items(&self) -> Result<Vec<PostMetadataCollectionItem>, CollectionError> {
        let Some(items) = self.core.items() else {
            // No metadata loaded yet - return empty list
            return Ok(Vec::new());
        };

        // Load ALL posts from cache - data availability is independent of EntityState.
        // After app restart, EntityState resets to Missing but data may still be cached.
        let all_ids: Vec<i64> = items.iter().map(|item| item.id()).collect();

        let cached_posts = if all_ids.is_empty() {
            Vec::new()
        } else {
            self.service
                .read_posts_by_ids_from_db(&all_ids)
                .map_err(|e| CollectionError::DatabaseError {
                    err_message: e.to_string(),
                })?
        };

        // Build a map for quick lookup (using remove to take ownership)
        let mut cached_map: std::collections::HashMap<i64, FullEntity<AnyPostWithEditContext>> =
            cached_posts.into_iter().map(|p| (p.data.id.0, p)).collect();

        // Combine EntityState with cache data into type-safe PostItemState
        let result = items
            .into_iter()
            .map(|item| {
                let id = item.id();
                // Extract parent and menu_order from metadata (available immediately)
                let parent = item.metadata.parent;
                let menu_order = item.metadata.menu_order;
                let cached_data = cached_map.remove(&id).map(|e| e.into());
                let state = match (item.state, cached_data) {
                    // Missing state
                    (EntityState::Missing, None) => PostItemState::Missing,
                    (EntityState::Missing, Some(data)) => PostItemState::Stale { data },

                    // Fetching state
                    (EntityState::Fetching, None) => PostItemState::Fetching,
                    (EntityState::Fetching, Some(data)) => PostItemState::FetchingWithData { data },

                    // Cached state (should always have data, but handle gracefully)
                    (EntityState::Cached, Some(data)) => PostItemState::Cached { data },
                    (EntityState::Cached, None) => PostItemState::Missing,

                    // Stale state (should always have data, but handle gracefully)
                    (EntityState::Stale, Some(data)) => PostItemState::Stale { data },
                    (EntityState::Stale, None) => PostItemState::Missing,

                    // Failed state
                    (EntityState::Failed { error }, None) => PostItemState::Failed { error },
                    (EntityState::Failed { error }, Some(data)) => {
                        PostItemState::FailedWithData { error, data }
                    }
                };

                PostMetadataCollectionItem {
                    id,
                    parent,
                    menu_order,
                    state,
                }
            })
            .collect();

        Ok(result)
    }

    /// Refresh the collection (fetch page 1, replace metadata).
    ///
    /// This:
    /// 1. Fetches metadata from the network (page 1)
    /// 2. Replaces existing metadata in the store
    /// 3. Fetches missing/stale entities
    ///
    /// Returns sync statistics including counts and pagination info.
    pub async fn refresh(&self) -> Result<SyncResult, FetchError> {
        log::debug!("PostMetadataCollection: Refreshing collection");

        let result = self
            .service
            .sync_list(
                self.core.key(),
                &self.endpoint_type,
                &self.filter,
                self.core.per_page(),
                true,
            )
            .await?;

        log::debug!(
            "PostMetadataCollection: Refreshed {} items, page 1 of {}, fetched {}, failed {}",
            result.total_items,
            result.total_pages.map(|p| p.to_string()).unwrap_or_else(|| "?".to_string()),
            result.fetched_count,
            result.failed_count
        );

        Ok(result)
    }

    /// Load the next page of items.
    ///
    /// This:
    /// 1. Fetches metadata for the next page
    /// 2. Appends to existing metadata in the store
    /// 3. Fetches missing/stale entities from the new page
    ///
    /// Returns `SyncResult::no_op()` if already on the last page.
    pub async fn load_next_page(&self) -> Result<SyncResult, FetchError> {
        let current_page = self.core.current_page();
        let total_pages = self.core.total_pages();

        // Check if no pages have been loaded yet (need refresh first)
        let Some(current_page) = current_page else {
            log::debug!("PostMetadataCollection: No pages loaded yet, need refresh first");
            return Ok(SyncResult::no_op(
                self.core.items().map(|items| items.len()).unwrap_or(0),
                Some(true), // has_more_pages = true, but need refresh first
                None,       // current_page = None (not loaded)
                None,
            ));
        };

        // Check if we're already at the last page (early exit for UX)
        if total_pages.is_some_and(|total| current_page >= total) {
            log::debug!("PostMetadataCollection: Already at last page, nothing to load");
            return Ok(SyncResult::no_op(
                self.core.items().map(|items| items.len()).unwrap_or(0),
                Some(false), // has_more_pages = false (on last page)
                Some(current_page),
                total_pages,
            ));
        }

        log::debug!("PostMetadataCollection: Loading next page");

        let result = self
            .service
            .sync_list(
                self.core.key(),
                &self.endpoint_type,
                &self.filter,
                self.core.per_page(),
                false,
            )
            .await?;

        log::debug!(
            "PostMetadataCollection: Loaded page {} of {}: {} items total, fetched {}, failed {}",
            result.current_page.map(|p| p.to_string()).unwrap_or_else(|| "?".to_string()),
            result.total_pages.map(|p| p.to_string()).unwrap_or_else(|| "?".to_string()),
            result.total_items,
            result.fetched_count,
            result.failed_count
        );

        Ok(result)
    }

    /// Get combined list info (pagination + sync state) in a single query.
    ///
    /// Returns `None` if the list hasn't been created yet.
    /// Use this instead of calling `current_page()`, `total_pages()`, `sync_state()`
    /// separately to avoid multiple database queries.
    pub fn list_info(&self) -> Option<ListInfo> {
        self.core.list_info()
    }

    /// Check if there are more pages to load.
    ///
    /// Returns:
    /// - `None` - Unknown (no metadata loaded or total_pages not provided by API)
    /// - `Some(true)` - More pages available
    /// - `Some(false)` - On last page
    pub fn has_more_pages(&self) -> Option<bool> {
        self.core.has_more_pages()
    }

    /// Get the current page number.
    ///
    /// Returns:
    /// - `None` - No metadata loaded yet
    /// - `Some(n)` - Currently on page n
    pub fn current_page(&self) -> Option<u32> {
        self.core.current_page()
    }

    /// Get the total number of pages, if known.
    pub fn total_pages(&self) -> Option<u32> {
        self.core.total_pages()
    }

    /// Get the current sync state for this collection.
    ///
    /// Returns the current `ListState`:
    /// - `Idle` - No sync in progress
    /// - `FetchingFirstPage` - Refresh in progress
    /// - `FetchingNextPage` - Load more in progress
    /// - `Error` - Last sync failed
    ///
    /// Use this to show loading indicators in the UI. Observe state changes
    /// via `is_relevant_state_update`.
    ///
    /// # Note
    /// This async function is exported to client platforms (Kotlin/Swift) where it
    /// will be executed on a background thread. The underlying Rust implementation
    /// is synchronous as rusqlite doesn't support async operations.
    pub async fn sync_state(&self) -> wp_mobile_cache::list_metadata::ListState {
        self.core.sync_state()
    }

    /// Check if a database update is relevant to this collection (either data or state).
    ///
    /// Returns `true` if the update affects either data or state.
    /// For more granular control, use `is_relevant_data_update` or `is_relevant_state_update`.
    pub fn is_relevant_update(&self, hook: &UpdateHook) -> bool {
        self.core.is_relevant_update(hook)
    }

    /// Check if a database update affects this collection's data.
    ///
    /// Returns `true` if the update is to:
    /// - An entity table this collection monitors (PostsEditContext, TermRelationships)
    /// - The ListMetadataItems table for this collection's key
    ///
    /// Use this for data observers that should refresh list contents.
    pub fn is_relevant_data_update(&self, hook: &UpdateHook) -> bool {
        self.core.is_relevant_data_update(hook)
    }

    /// Check if a database update affects this collection's list info (pagination + state).
    ///
    /// Returns `true` if the update is to:
    /// - `ListMetadata` table (pagination info changed)
    /// - `ListMetadataState` table (sync state changed)
    ///
    /// Use this for listInfo observers that should update pagination display and loading indicators.
    pub fn is_relevant_list_info_update(&self, hook: &UpdateHook) -> bool {
        self.core.is_relevant_list_info_update(hook)
    }

    /// Get the filter parameters for this collection.
    pub fn filter(&self) -> PostListFilter {
        self.filter.clone()
    }
}
