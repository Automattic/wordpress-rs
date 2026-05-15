//! Media-specific metadata collection for efficient list syncing.

use std::{collections::HashMap, sync::Arc};

use wp_api::media::MediaWithEditContext;
use wp_mobile_cache::{UpdateHook, entity::FullEntity};

use crate::{
    collection::{CollectionError, FetchError, MetadataCollectionCore},
    filters::MediaListFilter,
    service::media::MediaService,
    sync::{ListInfo, SyncResult},
};

// Generate MediaItemState enum, MediaMetadataCollectionItem struct, and From trait
// implementations using the shared macro. This mirrors the post metadata collection
// without duplicating the boilerplate.
crate::wp_mobile_metadata_item!(
    MediaMetadataCollectionItem,
    MediaItemState,
    crate::FullEntityMediaWithEditContext
);

/// Metadata-first collection for media with edit context.
///
/// This collection uses a two-phase sync strategy:
/// 1. Fetch lightweight metadata (id + modified_gmt) to define list structure
/// 2. Selectively fetch full data for missing or stale items
///
/// Mirrors `PostMetadataCollectionWithEditContext` but without the membership-update
/// path, since `MediaService` does not expose mutation methods that would
/// notify collections of changes.
#[derive(uniffi::Object)]
pub struct MediaMetadataCollectionWithEditContext {
    /// Core collection infrastructure (shared query logic)
    core: MetadataCollectionCore,

    /// Reference to service for sync operations and loading entity data
    service: Arc<MediaService>,

    /// Filter parameters for the media list
    filter: MediaListFilter,
}

impl MediaMetadataCollectionWithEditContext {
    pub fn new(
        core: MetadataCollectionCore,
        service: Arc<MediaService>,
        filter: MediaListFilter,
    ) -> Self {
        Self {
            core,
            service,
            filter,
        }
    }
}

#[uniffi::export]
impl MediaMetadataCollectionWithEditContext {
    /// Load all items with their current states and data.
    ///
    /// Returns items in list order with type-safe state representation.
    /// Each item's `state` is a [`MediaItemState`] variant that encodes both
    /// the sync status and data availability.
    ///
    /// This is the primary method for getting collection contents to display.
    ///
    /// # Note
    /// Data availability is independent of the internal `DbEntityState`. After an app
    /// restart, items may have internal state `Missing` but still have cached data
    /// available. This method will return `FetchingWithData`, `Stale`, or `FailedWithData`
    /// variants appropriately when cached data exists.
    ///
    /// This async function is exported to client platforms (Kotlin/Swift) where it
    /// will be executed on a background thread. The underlying Rust implementation
    /// is synchronous as rusqlite doesn't support async operations.
    pub async fn load_items(&self) -> Result<Vec<MediaMetadataCollectionItem>, CollectionError> {
        let Some(items) = self.core.items() else {
            // No metadata loaded yet - return empty list
            return Ok(Vec::new());
        };

        // Load ALL media from cache - data availability is independent of DbEntityState.
        // After app restart, DbEntityState resets to Missing but data may still be cached.
        let all_ids: Vec<i64> = items.iter().map(|item| item.id()).collect();

        let cached_media = if all_ids.is_empty() {
            Vec::new()
        } else {
            self.service
                .read_media_by_ids_from_db(&all_ids)
                .map_err(|e| CollectionError::DatabaseError {
                    err_message: e.to_string(),
                })?
        };

        // Build a map for quick lookup (using remove to take ownership)
        let mut cached_map: HashMap<i64, FullEntity<MediaWithEditContext>> =
            cached_media.into_iter().map(|m| (m.data.id.0, m)).collect();

        // Convert CollectionItem + cached data → MediaMetadataCollectionItem using From trait
        Ok(items
            .into_iter()
            .map(|item| {
                let id = item.id();
                let cached_data = cached_map.remove(&id).map(Into::into);
                MediaMetadataCollectionItem::from((item, cached_data))
            })
            .collect())
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
        log::debug!("MediaMetadataCollection: Refreshing collection");

        // If the per_page configuration changed since this list was created,
        // delete the stale list so it gets recreated with the correct per_page.
        // This is safe because refresh replaces all list content anyway.
        self.service
            .metadata_service
            .delete_list_if_per_page_changed(self.core.key(), self.core.per_page())?;

        let result = self
            .service
            .sync_list(self.core.key(), &self.filter, self.core.per_page(), true)
            .await?;

        log::debug!(
            "MediaMetadataCollection: Refreshed {} items, page 1 of {}, fetched {}, failed {}",
            result.total_items,
            result
                .total_pages
                .map(|p| p.to_string())
                .unwrap_or_else(|| "?".to_string()),
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
        // Delegate pagination logic to core orchestrator
        self.core
            .load_next_page_with(|| async {
                let result = self
                    .service
                    .sync_list(self.core.key(), &self.filter, self.core.per_page(), false)
                    .await?;

                log::debug!(
                    "MediaMetadataCollection: Loaded page {} of {}: {} items total, fetched {}, failed {}",
                    result
                        .current_page
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "?".to_string()),
                    result
                        .total_pages
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "?".to_string()),
                    result.total_items,
                    result.fetched_count,
                    result.failed_count
                );

                Ok(result)
            })
            .await
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
    /// - An entity table this collection monitors (MediaEditContext)
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
    pub fn filter(&self) -> MediaListFilter {
        self.filter.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::media::MediaService;
    use crate::testing::mock_api_client;
    use rstest::*;
    use rusqlite::Connection;
    use std::sync::Arc;
    use wp_api::api_client::WpApiClient;
    use wp_mobile_cache::{
        MigrationManager, WpApiCache, db_types::self_hosted_site::SelfHostedSite,
        repository::sites::SiteRepository,
    };

    /// Build a MediaService backed by an in-memory cache, mirroring the
    /// fixture pattern used by `service::media` tests.
    fn make_service(api_client: Arc<WpApiClient>) -> Arc<MediaService> {
        let mut conn = Connection::open_in_memory().expect("Failed to create in-memory database");
        let mut migration_manager =
            MigrationManager::new(&conn).expect("Failed to create migration manager");
        migration_manager
            .perform_migrations()
            .expect("Migrations should succeed");

        let site_repo = SiteRepository;
        let self_hosted_site = SelfHostedSite {
            url: "https://test.local".to_string(),
            api_root: "https://test.local/wp-json".to_string(),
        };
        let db_site = site_repo
            .upsert_self_hosted_site(&mut conn, &self_hosted_site)
            .expect("Site creation should succeed")
            .db_site;

        let cache = Arc::new(WpApiCache::try_from(conn).expect("Cache creation should succeed"));
        Arc::new(MediaService::new(api_client, Arc::new(db_site), cache))
    }

    /// Smoke test: load_items returns empty on a fresh cache and filter() returns
    /// the constructor-bound filter. Full sync/refresh flows are covered by the
    /// integration test in Task 8.
    #[rstest]
    #[tokio::test]
    async fn test_load_items_empty_and_filter_round_trip(mock_api_client: Arc<WpApiClient>) {
        let service = make_service(mock_api_client);
        let collection = service
            .create_media_metadata_collection_with_edit_context(MediaListFilter::default(), 10);

        let items = collection
            .load_items()
            .await
            .expect("load_items should succeed on fresh cache");
        assert!(items.is_empty(), "Fresh cache should yield no items");

        let filter = collection.filter();
        assert_eq!(filter, MediaListFilter::default());
    }
}
