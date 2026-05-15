//! Media-specific metadata collection for efficient list syncing.

use std::{cmp::Ordering, collections::HashMap, sync::Arc};

use wp_api::media::{MediaId, MediaWithEditContext};
use wp_mobile_cache::{
    UpdateHook,
    entity::FullEntity,
    repository::list_metadata::ListMetadataItemInput,
};

use crate::{
    collection::{CollectionError, FetchError, MetadataCollectionCore},
    filters::{MediaListFilter, compare_media_by_order},
    service::media::MediaService,
    sync::{EntityMetadata, ListInfo, SyncResult},
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
/// Mirrors `PostMetadataCollectionWithEditContext` including the membership-update
/// path.
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

/// Action to take when a media item's membership in a filtered list changes.
///
/// Mirrors `MembershipAction` in `post_metadata_collection.rs`. The two-variant
/// design is intentional: when local ordering cannot be resolved (non-deterministic
/// filter, missing sort keys), `compute_final_list` returns `None` and the caller
/// silently skips the list write — there is no third "needs refresh" variant.
enum MembershipAction {
    Remove,
    Insert,
}

// TODO: Remove allow(dead_code) once Task 4 wires notify_collections into
// production upload path, making key() and update_media_membership() reachable
// from non-test code.
#[allow(dead_code)]
impl MediaMetadataCollectionWithEditContext {
    /// Crate-internal accessor for the cache key. Used by service-side
    /// integration tests that seed `list_metadata_items` directly via the
    /// repo's `set_items_by_list_key` helper. Not exported to UniFFI.
    pub(crate) fn key(&self) -> wp_mobile_cache::list_metadata::ListKey {
        self.core.key().clone()
    }

    /// Update list membership for a single media item.
    ///
    /// Should be called after the media database is modified externally
    /// (e.g. by `MediaService::create_media`). Re-evaluates whether the media
    /// matches this collection's filter and updates the stored list accordingly.
    ///
    /// Mirrors `PostMetadataCollectionWithEditContext::update_post_membership`
    /// exactly. The two-variant `MembershipAction` plus `Option`-returning
    /// `compute_final_list` is the same pattern; under non-deterministic ordering
    /// (active search / relevance) this function silently skips the list write
    /// and the new item appears only after the next `refresh()`.
    pub(crate) fn update_media_membership(
        &self,
        media_id: MediaId,
    ) -> Result<(), CollectionError> {
        let media_id_i64 = media_id.0;

        // Phase 1: Lightweight membership check.
        let is_in_list = self
            .service
            .metadata_service
            .list_contains_entity(self.core.key(), media_id_i64)
            .map_err(|e| CollectionError::DatabaseError {
                err_message: e.to_string(),
            })?;

        let changed_media = self
            .service
            .read_media_by_ids_from_db(&[media_id_i64])
            .map_err(|e| CollectionError::DatabaseError {
                err_message: e.to_string(),
            })?
            .into_iter()
            .next()
            .map(|m| m.data);

        // Phase 2: Determine action.
        let action = if let Some(ref cached_media) = changed_media {
            let matches_filter = self.filter.loosely_matches_media(cached_media);
            if is_in_list && !matches_filter {
                Some(MembershipAction::Remove)
            } else if !is_in_list && matches_filter {
                Some(MembershipAction::Insert)
            } else {
                None
            }
        } else if is_in_list {
            // No longer in DB — remove from list.
            Some(MembershipAction::Remove)
        } else {
            None
        };

        let action = match action {
            Some(a) => a,
            None => return Ok(()),
        };

        // Phase 3: Apply.
        match action {
            MembershipAction::Remove => self
                .service
                .metadata_service
                .remove_list_items(self.core.key(), &[media_id_i64])
                .map_err(|e| CollectionError::DatabaseError {
                    err_message: e.to_string(),
                }),
            MembershipAction::Insert => self.insert_media_into_list(media_id_i64),
        }
    }

    /// Insert a media into the stored list, maintaining sort order.
    ///
    /// Mirrors `PostMetadataCollectionWithEditContext::insert_post_into_list`.
    fn insert_media_into_list(&self, media_id: i64) -> Result<(), CollectionError> {
        let current_metadata = self
            .service
            .metadata_service
            .get_metadata(self.core.key())
            .map_err(|e| CollectionError::DatabaseError {
                err_message: e.to_string(),
            })?
            .unwrap_or_default();
        let current_ids: Vec<i64> = current_metadata.iter().map(|m| m.id).collect();
        let metadata_by_id: HashMap<i64, &EntityMetadata> =
            current_metadata.iter().map(|m| (m.id, m)).collect();

        let all_media_ids: Vec<i64> = current_ids
            .iter()
            .copied()
            .chain(std::iter::once(media_id))
            .collect();
        let all_media = self
            .service
            .read_media_by_ids_from_db(&all_media_ids)
            .map_err(|e| CollectionError::DatabaseError {
                err_message: e.to_string(),
            })?;
        let media_by_id: HashMap<i64, MediaWithEditContext> =
            all_media.into_iter().map(|m| (m.data.id.0, m.data)).collect();

        let Some(media_to_insert) = media_by_id.get(&media_id) else {
            return Ok(());
        };

        let final_items = match self.compute_final_list(
            &current_ids,
            media_to_insert,
            &media_by_id,
            &metadata_by_id,
        ) {
            Some(items) => items,
            // Non-deterministic ordering or missing sort-key data: silently skip,
            // matching the post-side behaviour. The new media will appear on the
            // next explicit collection refresh.
            None => return Ok(()),
        };

        self.service
            .metadata_service
            .replace_list_items(self.core.key(), &final_items)
            .map_err(|e| CollectionError::DatabaseError {
                err_message: e.to_string(),
            })?;

        Ok(())
    }

    /// Build a `ListMetadataItemInput` from a media's cached data.
    fn media_to_item_input(media: &MediaWithEditContext) -> ListMetadataItemInput {
        ListMetadataItemInput {
            entity_id: media.id.0,
            modified_gmt: Some(media.modified_gmt.to_string()),
            parent: media.post_id.map(|p| p.0),
            menu_order: None,
        }
    }

    /// Build a `ListMetadataItemInput` for an entity ID, falling back from fresh
    /// data to stored metadata to minimal entry. Mirrors the post-side helper.
    fn id_to_item_input(
        id: i64,
        media_by_id: &HashMap<i64, MediaWithEditContext>,
        metadata_by_id: &HashMap<i64, &EntityMetadata>,
    ) -> ListMetadataItemInput {
        if let Some(media) = media_by_id.get(&id) {
            Self::media_to_item_input(media)
        } else if let Some(metadata) = metadata_by_id.get(&id) {
            ListMetadataItemInput {
                entity_id: id,
                modified_gmt: metadata.modified_gmt.as_ref().map(|d| d.to_string()),
                parent: metadata.parent,
                menu_order: metadata.menu_order,
            }
        } else {
            ListMetadataItemInput {
                entity_id: id,
                modified_gmt: None,
                parent: None,
                menu_order: None,
            }
        }
    }

    /// Compute the final ordered list by merging retained items with new insertion.
    ///
    /// Returns `None` if ordering cannot be determined (non-deterministic filter
    /// or missing sort-key data), signaling that a full list refresh is needed.
    /// Caller silently skips the list write in that case.
    fn compute_final_list(
        &self,
        current_ids: &[i64],
        media_to_insert: &MediaWithEditContext,
        media_by_id: &HashMap<i64, MediaWithEditContext>,
        metadata_by_id: &HashMap<i64, &EntityMetadata>,
    ) -> Option<Vec<ListMetadataItemInput>> {
        if !self.filter.has_deterministic_ordering() {
            return None;
        }

        let orderby = self.filter.effective_orderby();
        let order = self.filter.effective_order();

        // Find where the new media belongs in the existing list.
        let mut insert_at = current_ids.len();
        for (i, &id) in current_ids.iter().enumerate() {
            let existing = media_by_id.get(&id)?;
            match compare_media_by_order(media_to_insert, existing, orderby, order) {
                Some(Ordering::Less) => {
                    insert_at = i;
                    break;
                }
                Some(_) => {}
                None => return None,
            }
        }

        let mut result: Vec<_> = current_ids
            .iter()
            .map(|&id| Self::id_to_item_input(id, media_by_id, metadata_by_id))
            .collect();
        result.insert(insert_at, Self::media_to_item_input(media_to_insert));

        Some(result)
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
