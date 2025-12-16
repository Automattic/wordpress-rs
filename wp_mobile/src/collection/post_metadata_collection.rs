//! Post-specific metadata collection for efficient list syncing.

use std::sync::Arc;

use wp_api::posts::{AnyPostWithEditContext, PostListParams};
use wp_mobile_cache::{UpdateHook, entity::FullEntity};

use crate::{
    collection::{CollectionError, FetchError},
    service::posts::PostService,
    sync::{
        EntityState, ListInfo, MetadataCollection, PersistentPostMetadataFetcherWithEditContext,
        SyncResult,
    },
};

// Generate PostItemState enum using the macro
crate::wp_mobile_item_state!(PostItemState, crate::FullEntityAnyPostWithEditContext);

/// Item in a metadata collection with type-safe state representation.
///
/// The `state` enum encodes both the sync status and data availability,
/// making it impossible to have inconsistent combinations.
#[derive(uniffi::Record)]
pub struct PostMetadataCollectionItem {
    /// The post ID
    pub id: i64,

    /// Combined state and data - see [`PostItemState`] for variants
    pub state: PostItemState,
}

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
/// let collection = post_service.create_post_metadata_collection_with_edit_context(params);
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
    /// The underlying metadata collection (database-backed)
    collection: MetadataCollection<PersistentPostMetadataFetcherWithEditContext>,

    /// Reference to service for loading full entity data
    post_service: Arc<PostService>,

    /// The API parameters for this collection
    params: PostListParams,
}

impl PostMetadataCollectionWithEditContext {
    pub fn new(
        collection: MetadataCollection<PersistentPostMetadataFetcherWithEditContext>,
        post_service: Arc<PostService>,
        params: PostListParams,
    ) -> Self {
        Self {
            collection,
            post_service,
            params,
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
        let items = self.collection.items();

        // Load ALL posts from cache - data availability is independent of EntityState.
        // After app restart, EntityState resets to Missing but data may still be cached.
        let all_ids: Vec<i64> = items.iter().map(|item| item.id()).collect();

        let cached_posts = if all_ids.is_empty() {
            Vec::new()
        } else {
            self.post_service
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

                PostMetadataCollectionItem { id, state }
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
        self.collection.refresh().await
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
        self.collection.load_next_page().await
    }

    /// Get combined list info (pagination + sync state) in a single query.
    ///
    /// Returns `None` if the list hasn't been created yet.
    /// Use this instead of calling `current_page()`, `total_pages()`, `sync_state()`
    /// separately to avoid multiple database queries.
    pub fn list_info(&self) -> Option<ListInfo> {
        self.collection.list_info()
    }

    /// Check if there are more pages to load.
    pub fn has_more_pages(&self) -> bool {
        self.collection.has_more_pages()
    }

    /// Get the current page number (0 = not loaded yet).
    pub fn current_page(&self) -> u32 {
        self.collection.current_page()
    }

    /// Get the total number of pages, if known.
    pub fn total_pages(&self) -> Option<u32> {
        self.collection.total_pages()
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
        self.collection.sync_state()
    }

    /// Check if a database update is relevant to this collection (either data or state).
    ///
    /// Returns `true` if the update affects either data or state.
    /// For more granular control, use `is_relevant_data_update` or `is_relevant_state_update`.
    pub fn is_relevant_update(&self, hook: &UpdateHook) -> bool {
        self.collection.is_relevant_update(hook)
    }

    /// Check if a database update affects this collection's data.
    ///
    /// Returns `true` if the update is to:
    /// - An entity table this collection monitors (PostsEditContext, TermRelationships)
    /// - The ListMetadataItems table for this collection's key
    ///
    /// Use this for data observers that should refresh list contents.
    pub fn is_relevant_data_update(&self, hook: &UpdateHook) -> bool {
        self.collection.is_relevant_data_update(hook)
    }

    /// Check if a database update affects this collection's list info (pagination + state).
    ///
    /// Returns `true` if the update is to:
    /// - `ListMetadata` table (pagination info changed)
    /// - `ListMetadataState` table (sync state changed)
    ///
    /// Use this for listInfo observers that should update pagination display and loading indicators.
    pub fn is_relevant_list_info_update(&self, hook: &UpdateHook) -> bool {
        self.collection.is_relevant_list_info_update(hook)
    }

    /// Get the API parameters for this collection.
    pub fn params(&self) -> PostListParams {
        self.params.clone()
    }
}
