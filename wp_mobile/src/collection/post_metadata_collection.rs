//! Post-specific metadata collection for efficient list syncing.

use std::{collections::HashMap, sync::Arc};

use wp_api::posts::AnyPostWithEditContext;
use wp_api::request::endpoint::posts_endpoint::PostEndpointType;
use wp_mobile_cache::{
    UpdateHook, entity::FullEntity, repository::list_metadata::ListMetadataItemInput,
};

use crate::{
    collection::{CollectionError, FetchError, MetadataCollectionCore},
    filters::PostListFilter,
    service::posts::PostService,
    sync::{EntityMetadata, ListInfo, SyncResult},
};

// Generate PostItemState enum, PostMetadataCollectionItem struct, and From trait implementations
// using the macro. This eliminates ~63 lines of boilerplate that would be duplicated across
// all entity types (categories, users, comments, etc.).
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
///         PostItemState::Fresh { data } => { /* show data */ }
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
    /// Data availability is independent of the internal `DbEntityState`. After an app
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

        // Load ALL posts from cache - data availability is independent of DbEntityState.
        // After app restart, DbEntityState resets to Missing but data may still be cached.
        let all_ids: Vec<i64> = items.iter().map(|item| item.id()).collect();

        let cached_posts = if all_ids.is_empty() {
            Vec::new()
        } else {
            self.service
                .read_post_full_entities_by_ids_from_db(&all_ids)
                .map_err(|e| CollectionError::DatabaseError {
                    err_message: e.to_string(),
                })?
        };

        // Build a map for quick lookup (using remove to take ownership)
        let mut cached_map: HashMap<i64, FullEntity<AnyPostWithEditContext>> =
            cached_posts.into_iter().map(|p| (p.data.id.0, p)).collect();

        // Convert CollectionItem + cached data → PostMetadataCollectionItem using From trait
        Ok(items
            .into_iter()
            .map(|item| {
                let id = item.id();
                let cached_data = cached_map.remove(&id).map(Into::into);
                PostMetadataCollectionItem::from((item, cached_data))
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
        log::debug!("PostMetadataCollection: Refreshing collection");

        // If the per_page configuration changed since this list was created,
        // delete the stale list so it gets recreated with the correct per_page.
        // This is safe because refresh replaces all list content anyway.
        self.service
            .metadata_service
            .delete_list_if_per_page_changed(self.core.key(), self.core.per_page())?;

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

/// Action to take when a post's membership in a filtered list changes.
enum MembershipAction {
    Remove,
    Insert,
    Update,
}

impl PostMetadataCollectionWithEditContext {
    /// Update list membership for a single post.
    ///
    /// Should be called after the post database is modified externally
    /// (e.g. by `PostService`). Re-evaluates whether the post matches this
    /// collection's filter and updates the stored list accordingly
    /// (adding or removing the post).
    ///
    /// Note: this function does not update `ListInfo` (e.g. `total_items`,
    /// `total_pages`). Those values come from API response headers and
    /// would need a refresh to be accurate.
    ///
    /// Note: this function is not synchronized with `refresh` or
    /// `load_next_page`. The implementation minimises overwrites by using
    /// targeted deletes and fresh reads, so races are possible but rare.
    ///
    /// # Arguments
    /// * `post_id` - WordPress post ID that changed
    pub(crate) fn update_post_membership(&self, post_id: i64) -> Result<(), CollectionError> {
        // Phase 1: Lightweight membership check: only check whether this
        // single post is in the list, rather than fetching all entity IDs.
        let is_in_list = self
            .service
            .metadata_service
            .list_contains_entity(self.core.key(), post_id)
            .map_err(|e| CollectionError::DatabaseError {
                err_message: e.to_string(),
            })?;

        let changed_post = self
            .service
            .read_post_full_entities_by_ids_from_db(&[post_id])
            .map_err(|e| CollectionError::DatabaseError {
                err_message: e.to_string(),
            })?
            .into_iter()
            .next()
            .map(|p| p.data);

        // Phase 2: Determine action
        let action = if let Some(ref cached_post) = changed_post {
            let matches_filter = self.filter.loosely_matches_post(cached_post);
            if is_in_list && !matches_filter {
                Some(MembershipAction::Remove)
            } else if !is_in_list && matches_filter {
                Some(MembershipAction::Insert)
            } else if is_in_list && matches_filter {
                Some(MembershipAction::Update)
            } else {
                None
            }
        } else if is_in_list {
            // The post no longer exists in the database: remove it from the list.
            Some(MembershipAction::Remove)
        } else {
            None
        };

        let action = match action {
            Some(a) => a,
            None => return Ok(()),
        };

        // Phase 3: Apply the membership change to the stored list.
        match action {
            MembershipAction::Remove => {
                // Targeted DELETE to avoid overwriting when possible.
                self.service
                    .metadata_service
                    .remove_list_items(self.core.key(), &[post_id])
                    .map_err(|e| CollectionError::DatabaseError {
                        err_message: e.to_string(),
                    })
            }
            MembershipAction::Insert | MembershipAction::Update => {
                self.insert_post_into_list(post_id)
            }
        }
    }

    /// Insert or update a post in the stored list, maintaining sort order.
    ///
    /// Reads full metadata fresh so we have the latest snapshot, then
    /// rewrites the list to maintain sort order via rowid.
    fn insert_post_into_list(&self, post_id: i64) -> Result<(), CollectionError> {
        let current_metadata = self
            .service
            .metadata_service
            .get_metadata(self.core.key())
            .map_err(|e| CollectionError::DatabaseError {
                err_message: e.to_string(),
            })?
            .unwrap_or_default();
        let current_ids: Vec<i64> = current_metadata
            .iter()
            .map(|m| m.id)
            .filter(|&id| id != post_id)
            .collect();
        let metadata_by_id: HashMap<i64, &EntityMetadata> =
            current_metadata.iter().map(|m| (m.id, m)).collect();

        // May contain fewer posts than `current_ids` if some have been
        // deleted or not yet fetched.
        let all_post_ids: Vec<i64> = current_ids
            .iter()
            .copied()
            .chain(std::iter::once(post_id))
            .collect();
        let all_posts = self
            .service
            .read_post_full_entities_by_ids_from_db(&all_post_ids)
            .map_err(|e| CollectionError::DatabaseError {
                err_message: e.to_string(),
            })?;
        let posts_by_id: HashMap<i64, AnyPostWithEditContext> = all_posts
            .into_iter()
            .map(|p| (p.data.id.0, p.data))
            .collect();

        let Some(post_to_insert) = posts_by_id.get(&post_id) else {
            return Ok(());
        };

        let final_items = match self.compute_final_list(
            &current_ids,
            post_to_insert,
            &posts_by_id,
            &metadata_by_id,
        ) {
            Some(items) => items,
            None => {
                // If ordering cannot be determined locally (e.g. relevance search or missing
                // sort-key data for existing items), and the post was already in the list,
                // update its metadata in-place preserving its existing position.
                if current_metadata.iter().any(|m| m.id == post_id) {
                    current_metadata
                        .iter()
                        .map(|m| {
                            if m.id == post_id {
                                Self::post_to_item_input(post_to_insert)
                            } else {
                                Self::id_to_item_input(m.id, &posts_by_id, &metadata_by_id)
                            }
                        })
                        .collect()
                } else {
                    return Ok(());
                }
            }
        };

        self.service
            .metadata_service
            .replace_list_items(self.core.key(), &final_items)
            .map_err(|e| CollectionError::DatabaseError {
                err_message: e.to_string(),
            })?;

        Ok(())
    }

    /// Build a `ListMetadataItemInput` from a post's cached data.
    fn post_to_item_input(post: &AnyPostWithEditContext) -> ListMetadataItemInput {
        ListMetadataItemInput {
            entity_id: post.id.0,
            modified_gmt: Some(post.modified_gmt),
            parent: post.parent.map(|p| p.0),
            menu_order: post.menu_order,
        }
    }

    /// Build a `ListMetadataItemInput` for an entity ID.
    ///
    /// Prefers fresh data from `posts_by_id`, falls back to previously stored
    /// list metadata, and finally to a minimal item (entity_id only).
    fn id_to_item_input(
        id: i64,
        posts_by_id: &HashMap<i64, AnyPostWithEditContext>,
        metadata_by_id: &HashMap<i64, &EntityMetadata>,
    ) -> ListMetadataItemInput {
        if let Some(post) = posts_by_id.get(&id) {
            Self::post_to_item_input(post)
        } else if let Some(metadata) = metadata_by_id.get(&id) {
            ListMetadataItemInput {
                entity_id: id,
                modified_gmt: metadata.modified_gmt,
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

    /// Compute the final ordered list by merging retained items with new insertions.
    ///
    /// Returns `None` if ordering cannot be determined (non-deterministic filter
    /// or missing sort key data), signaling that a full list refresh is needed.
    fn compute_final_list(
        &self,
        current_ids: &[i64],
        post_to_insert: &AnyPostWithEditContext,
        posts_by_id: &HashMap<i64, AnyPostWithEditContext>,
        metadata_by_id: &HashMap<i64, &EntityMetadata>,
    ) -> Option<Vec<ListMetadataItemInput>> {
        if !self.filter.has_deterministic_ordering() {
            return None;
        }

        let orderby = self.filter.effective_orderby();
        let order = self.filter.effective_order();

        // Find where the new post belongs in the existing list.
        let mut insert_at = current_ids.len();
        for (i, &id) in current_ids.iter().enumerate() {
            let existing_post = posts_by_id.get(&id)?;
            match crate::filters::compare_posts_by_order(
                post_to_insert,
                existing_post,
                orderby,
                order,
            ) {
                Some(std::cmp::Ordering::Less) => {
                    insert_at = i;
                    break;
                }
                Some(_) => {}
                None => return None,
            }
        }

        let mut result: Vec<_> = current_ids
            .iter()
            .map(|&id| Self::id_to_item_input(id, posts_by_id, metadata_by_id))
            .collect();
        result.insert(insert_at, Self::post_to_item_input(post_to_insert));

        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::{CollectionItem, DbEntityState};
    use wp_api::prelude::WpGmtDateTime;

    #[test]
    fn test_post_metadata_collection_item_from_collection_item_with_modified_gmt() {
        let modified_gmt = "2026-10-20 12:34:56"
            .parse::<WpGmtDateTime>()
            .expect("Valid datetime string should parse");
        let metadata = EntityMetadata::new(42, Some(modified_gmt), Some(10), Some(5));
        let collection_item = CollectionItem::new(metadata, DbEntityState::Fresh);

        let item = PostMetadataCollectionItem::from((collection_item, None));

        assert_eq!(item.id, 42);
        assert_eq!(item.modified_gmt, Some(modified_gmt));
        assert_eq!(item.parent, Some(10));
        assert_eq!(item.menu_order, Some(5));
        assert!(matches!(item.state, PostItemState::Missing));
    }

    #[test]
    fn test_post_metadata_collection_item_from_collection_item_without_modified_gmt() {
        let metadata = EntityMetadata::new(42, None, None, None);
        let collection_item = CollectionItem::new(metadata, DbEntityState::Missing);

        let item = PostMetadataCollectionItem::from((collection_item, None));

        assert_eq!(item.id, 42);
        assert_eq!(item.modified_gmt, None);
        assert_eq!(item.parent, None);
        assert_eq!(item.menu_order, None);
        assert!(matches!(item.state, PostItemState::Missing));
    }

    #[test]
    fn test_post_metadata_collection_item_retains_list_metadata_time_when_entity_time_differs() {
        let list_time = "2026-10-20 12:00:00"
            .parse::<WpGmtDateTime>()
            .expect("Valid datetime string should parse");
        let post_time = "2026-10-20 15:00:00"
            .parse::<WpGmtDateTime>()
            .expect("Valid datetime string should parse");

        let metadata = EntityMetadata::new(42, Some(list_time), Some(10), Some(5));
        let collection_item = CollectionItem::new(metadata, DbEntityState::Stale);

        let mut post = wp_mobile_cache::test_fixtures::posts::PostBuilder::minimal()
            .with_id(42)
            .build();
        post.modified_gmt = post_time;
        let full_entity = crate::FullEntityAnyPostWithEditContext {
            entity_id: wp_mobile_cache::entity::EntityId {
                db_site: wp_mobile_cache::db_types::db_site::DbSite {
                    row_id: wp_mobile_cache::RowId(1),
                    site_type: wp_mobile_cache::db_types::db_site::DbSiteType::SelfHosted,
                    mapped_site_id: wp_mobile_cache::RowId(1),
                },
                table: wp_mobile_cache::DbTable::PostsEditContext,
                rowid: wp_mobile_cache::RowId(1),
            },
            data: post,
        };

        let item = PostMetadataCollectionItem::from((collection_item, Some(full_entity)));

        assert_eq!(item.id, 42);
        assert_eq!(item.modified_gmt, Some(list_time));
        if let PostItemState::Stale { data } = item.state {
            assert_eq!(data.data.modified_gmt, post_time);
        } else {
            panic!("Expected Stale state, got {:?}", item.state);
        }
    }

    #[tokio::test]
    async fn test_modified_gmt_updated_after_local_edit_of_listed_post() {
        use crate::service::posts::PostService;
        use crate::testing::mock_api_client;
        use rusqlite::Connection;
        use std::sync::Arc;
        use wp_mobile_cache::context::EditContext;
        use wp_mobile_cache::{
            MigrationManager, WpApiCache,
            db_types::self_hosted_site::SelfHostedSite,
            repository::{
                list_metadata::ListMetadataRepository, posts::PostRepository, sites::SiteRepository,
            },
            test_fixtures::posts::PostBuilder,
        };

        let mut conn = Connection::open_in_memory().unwrap();
        MigrationManager::new(&conn)
            .unwrap()
            .perform_migrations()
            .unwrap();
        let db_site = SiteRepository
            .upsert_self_hosted_site(
                &mut conn,
                &SelfHostedSite {
                    url: "https://test.local".to_string(),
                    api_root: "https://test.local/wp-json".to_string(),
                },
            )
            .unwrap()
            .db_site;
        let cache = Arc::new(WpApiCache::try_from(conn).unwrap());
        let db_site = Arc::new(db_site);
        let service = Arc::new(PostService::new(
            mock_api_client(),
            db_site.clone(),
            cache.clone(),
        ));
        let collection = service.create_post_metadata_collection_with_edit_context(
            PostEndpointType::Posts,
            PostListFilter::default(),
            20,
        );

        let t1: WpGmtDateTime = "2024-01-01T00:00:00Z".parse().unwrap();
        let t2: WpGmtDateTime = "2026-11-04T12:00:00Z".parse().unwrap();

        // List was synced while the post's modified_gmt was T1.
        let mut post = PostBuilder::minimal().with_id(42).build();
        post.modified_gmt = t1;
        cache
            .execute(|conn| {
                PostRepository::<EditContext>::new().upsert(conn, &db_site, &post)?;
                let list_id = ListMetadataRepository::get_or_create(
                    conn,
                    &db_site,
                    collection.core.key(),
                    20,
                )?;
                ListMetadataRepository::set_items_by_list_metadata_id(
                    conn,
                    list_id,
                    &[ListMetadataItemInput {
                        entity_id: 42,
                        modified_gmt: Some(t1),
                        parent: None,
                        menu_order: None,
                    }],
                )
            })
            .unwrap();

        // The user edits the post: the server returns modified_gmt = T2, which
        // update_post upserts and then calls notify_collections(42).
        post.modified_gmt = t2;
        cache
            .execute(|conn| PostRepository::<EditContext>::new().upsert(conn, &db_site, &post))
            .unwrap();
        collection.update_post_membership(42).unwrap();

        let items = collection.load_items().await.unwrap();
        assert_eq!(items.len(), 1);
        let data_modified_gmt = match &items[0].state {
            PostItemState::FetchingWithData { data }
            | PostItemState::Fresh { data }
            | PostItemState::Stale { data }
            | PostItemState::FailedWithData { data, .. } => data.data.modified_gmt,
            other => panic!("expected cached data, got {other:?}"),
        };

        assert_eq!(
            data_modified_gmt, t2,
            "cached post has the edited timestamp"
        );
        assert_eq!(
            items[0].modified_gmt,
            Some(t2),
            "item.modified_gmt should match the edited post, but the list metadata row was never updated"
        );
    }
}
