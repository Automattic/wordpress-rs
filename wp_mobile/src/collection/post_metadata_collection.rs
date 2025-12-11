//! Post-specific metadata collection for efficient list syncing.

use std::sync::Arc;

use wp_api::posts::AnyPostWithEditContext;
use wp_mobile_cache::{UpdateHook, entity::FullEntity};

use crate::{
    collection::{CollectionError, FetchError},
    filters::AnyPostFilter,
    service::posts::PostService,
    sync::{
        EntityState, MetadataCollection, PersistentPostMetadataFetcherWithEditContext, SyncResult,
    },
};

/// Item in a metadata collection with optional loaded data.
///
/// Combines the collection item (id + state) with the full entity data
/// when available (i.e., when state is Cached).
// TODO: Move state representation to Rust with proper enum modeling.
// See metadata_collection_v3.md "TODO: Refined State Representation"
// Current design uses separate fields; should be a sealed enum for type safety.
#[derive(uniffi::Record)]
pub struct PostMetadataCollectionItem {
    /// The post ID
    pub id: i64,

    /// Current fetch state
    pub state: EntityState,

    /// Full entity data, present when state is Cached
    /// None for Missing, Fetching, or Failed states
    pub data: Option<crate::FullEntityAnyPostWithEditContext>,
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
/// let collection = post_service.create_post_metadata_collection_with_edit_context(filter);
///
/// // Initial load - fetches metadata, then syncs missing items
/// collection.refresh().await?;
///
/// // Get items with states and data
/// let items = collection.load_items()?;
/// for item in items {
///     match item.state {
///         EntityState::Cached => { /* show item.data */ }
///         EntityState::Fetching => { /* show loading */ }
///         EntityState::Failed { .. } => { /* show error */ }
///         _ => { /* show placeholder */ }
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

    /// The filter for this collection
    filter: AnyPostFilter,
}

impl PostMetadataCollectionWithEditContext {
    pub fn new(
        collection: MetadataCollection<PersistentPostMetadataFetcherWithEditContext>,
        post_service: Arc<PostService>,
        filter: AnyPostFilter,
    ) -> Self {
        Self {
            collection,
            post_service,
            filter,
        }
    }
}

#[uniffi::export]
impl PostMetadataCollectionWithEditContext {
    /// Load all items with their current states and data.
    ///
    /// Returns items in list order with:
    /// - `id`: The post ID
    /// - `state`: Current fetch state (Missing, Fetching, Cached, Stale, Failed)
    /// - `data`: Full entity data when state is Cached, None otherwise
    ///
    /// This is the primary method for getting collection contents to display.
    ///
    /// # Note
    /// This async function is exported to client platforms (Kotlin/Swift) where it
    /// will be executed on a background thread. The underlying Rust implementation
    /// is synchronous as rusqlite doesn't support async operations.
    pub async fn load_items(&self) -> Result<Vec<PostMetadataCollectionItem>, CollectionError> {
        let items = self.collection.items();

        // Load all cached posts in one query
        let cached_ids: Vec<i64> = items
            .iter()
            .filter(|item| item.state.is_cached())
            .map(|item| item.id())
            .collect();

        let cached_posts = if cached_ids.is_empty() {
            Vec::new()
        } else {
            self.post_service
                .read_posts_by_ids_from_db(&cached_ids)
                .map_err(|e| CollectionError::DatabaseError {
                    err_message: e.to_string(),
                })?
        };

        // Build a map for quick lookup (using remove to take ownership)
        let mut cached_map: std::collections::HashMap<i64, FullEntity<AnyPostWithEditContext>> =
            cached_posts.into_iter().map(|p| (p.data.id.0, p)).collect();

        // Combine items with their data
        let result = items
            .into_iter()
            .map(|item| {
                let data = if item.state.is_cached() {
                    cached_map.remove(&item.id()).map(|e| e.into())
                } else {
                    None
                };

                PostMetadataCollectionItem {
                    id: item.id(),
                    state: item.state,
                    data,
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

    /// Check if a database update affects this collection's sync state.
    ///
    /// Returns `true` if the update is to the ListMetadataState table
    /// for this collection's specific list.
    ///
    /// Use this for state observers that should update loading indicators.
    pub fn is_relevant_state_update(&self, hook: &UpdateHook) -> bool {
        self.collection.is_relevant_state_update(hook)
    }

    /// Get the filter for this collection.
    pub fn filter(&self) -> AnyPostFilter {
        self.filter.clone()
    }
}
