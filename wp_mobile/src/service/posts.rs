use crate::{
    AllAnyPostWithEditContextCollection, EntityAnyPostWithEditContext,
    PostCollectionWithEditContext,
    cache_key::{endpoint_type_cache_key, post_list_filter_cache_key},
    collection::{
        FetchError, FetchResult, PostMetadataCollectionWithEditContext, StatelessCollection,
        post_collection::PostCollection,
    },
    filters::{AnyPostFilter, PostListFilter},
    service::metadata::MetadataService,
    sync::{
        EntityMetadata, EntityState, EntityStateReader, EntityStateStore, MetadataCollectionCore,
        MetadataFetchResult, SyncResult, SyncStrategy,
    },
};
use std::{collections::HashSet, sync::Arc};
use wp_api::{
    api_client::WpApiClient,
    posts::{
        AnyPostWithEditContext, PostId, PostListParams, PostStatus,
        SparseAnyPostFieldWithEditContext,
    },
    request::endpoint::posts_endpoint::PostEndpointType,
};
use wp_mobile_cache::{
    DbTable, WpApiCache,
    context::EditContext,
    db_types::db_site::DbSite,
    entity::{Entity, EntityId, FullEntity},
    list_metadata::ListKey,
    repository::posts::PostRepository,
};

/// Maximum number of posts to fetch in a single batch request
const BATCH_FETCH_SIZE: usize = 100;

// Internal types

/// Result from fetching posts by IDs.
pub struct FetchByIdsResult {
    /// Entity IDs of successfully fetched posts
    pub entity_ids: Vec<EntityId>,
    /// Number of posts that were requested but failed to fetch
    pub failed_count: usize,
}

/// Statistics from fetching missing and stale posts.
pub(crate) struct FetchStats {
    /// Number of posts that needed fetching (Missing or Stale state)
    pub(crate) fetched_count: usize,
    /// Number of posts that failed to fetch
    pub(crate) failed_count: usize,
}

/// Service layer for post operations
///
/// Provides a bridge between clients and the underlying network/cache layers.
/// Handles fetching, creating, updating, and deleting posts.
///
/// # Metadata Sync Infrastructure
///
/// The service owns shared stores for metadata-first sync:
/// - `state_store_with_edit_context`: Tracks fetch state per entity for edit context.
///   Each context needs its own state store since the same entity ID can have different
///   fetch states across contexts.
/// - `metadata_service`: Database-backed list metadata (persists across app restarts).
///
/// Collections get read-only access via reader methods. This ensures cross-collection
/// consistency when multiple collections share the same underlying entities.
#[derive(uniffi::Object)]
pub struct PostService {
    db_site: Arc<DbSite>,
    api_client: Arc<WpApiClient>,
    cache: Arc<WpApiCache>,

    /// Per-entity fetch state for edit context (memory-only, resets on app restart).
    /// Each context needs its own state store since the same entity ID can have
    /// different fetch states across contexts.
    state_store_with_edit_context: Arc<EntityStateStore>,

    /// Database-backed list metadata service.
    /// Persists list structure across app restarts.
    metadata_service: Arc<MetadataService>,
}

impl PostService {
    pub fn new(api_client: Arc<WpApiClient>, db_site: Arc<DbSite>, cache: Arc<WpApiCache>) -> Self {
        let metadata_service = Arc::new(MetadataService::new(db_site.clone(), cache.clone()));

        Self {
            api_client,
            db_site,
            cache,
            state_store_with_edit_context: Arc::new(EntityStateStore::new()),
            metadata_service,
        }
    }

    /// Sync a page of posts from network to cache.
    ///
    /// Fetches full post data from the API and saves it to the database:
    /// 1. Converts filter to API parameters
    /// 2. Makes network request via WpApiClient
    /// 3. Upserts posts to database via repository
    /// 4. Returns entity IDs and pagination info
    ///
    /// # Arguments
    /// * `filter` - Post filter criteria
    /// * `page` - Page number to fetch (1-indexed)
    /// * `per_page` - Number of posts per page
    ///
    /// # Returns
    /// - `Ok(FetchResult)` with entity IDs of saved posts
    /// - `Err(FetchError)` if network or database error occurs
    ///
    /// # Database Updates
    /// Triggers database update hooks which notify observers watching the relevant tables.
    pub async fn sync_posts_page(
        &self,
        filter: &AnyPostFilter,
        page: u32,
        per_page: u32,
    ) -> Result<FetchResult, FetchError> {
        // Convert filter to API params
        let mut params = filter.to_list_params();
        params.page = Some(page);
        params.per_page = Some(per_page);

        // Make network request
        let response = self
            .api_client
            .posts()
            .list_with_edit_context(&PostEndpointType::Posts, &params)
            .await?;

        // Upsert to database and collect entity IDs
        let entity_ids = self.cache.execute(|conn| {
            let repo = PostRepository::<EditContext>::new();
            response
                .data
                .iter()
                .map(|post| {
                    repo.upsert(conn, &self.db_site, post)
                        .map_err(|e| FetchError::Database {
                            err_message: e.to_string(),
                        })
                })
                .collect::<Result<Vec<EntityId>, FetchError>>()
        })?;

        Ok(FetchResult {
            entity_ids,
            total_items: response.header_map.wp_total().map(|n| n as i64),
            total_pages: response.header_map.wp_total_pages(),
            current_page: page,
        })
    }

    /// Fetch lightweight metadata (id, modified_gmt, parent, menu_order) for a page of posts.
    ///
    /// Returns only the minimal fields needed to determine list structure and staleness.
    /// Does not fetch or save full post content.
    pub(crate) async fn fetch_posts_metadata(
        &self,
        endpoint_type: &PostEndpointType,
        filter: &PostListFilter,
        page: u32,
        per_page: u32,
    ) -> Result<MetadataFetchResult, FetchError> {
        // Convert filter to params with pagination
        let request_params = filter.to_list_params(page, per_page);

        let response = self
            .api_client
            .posts()
            .filter_list_with_edit_context(
                endpoint_type,
                &request_params,
                &[
                    SparseAnyPostFieldWithEditContext::Id,
                    SparseAnyPostFieldWithEditContext::ModifiedGmt,
                    SparseAnyPostFieldWithEditContext::Parent,
                    SparseAnyPostFieldWithEditContext::MenuOrder,
                ],
            )
            .await?;

        // Map sparse posts to EntityMetadata, filtering out any with missing id
        let metadata: Vec<EntityMetadata> = response
            .data
            .into_iter()
            .filter_map(|sparse| {
                Some(EntityMetadata::new(
                    sparse.id?.0,
                    sparse.modified_gmt,
                    sparse.parent.map(|p| p.0),
                    sparse.menu_order.map(|m| m as i64),
                ))
            })
            .collect();

        Ok(MetadataFetchResult::new(
            metadata,
            response.header_map.wp_total().map(|n| n as i64),
            response.header_map.wp_total_pages(),
            page,
        ))
    }

    /// Find stale posts by comparing fetched metadata timestamps with cached DB values.
    ///
    /// A post is considered stale if:
    /// 1. It's currently in `Cached` state in the state store
    /// 2. Its fetched `modified_gmt` differs from the cached `modified_gmt` in the database
    ///
    /// Returns empty vector if no stale posts found or if DB query fails.
    pub(crate) fn find_stale_posts_by_timestamp(
        &self,
        metadata: &[EntityMetadata],
        state_reader: &dyn EntityStateReader,
    ) -> Vec<i64> {
        // Filter to only posts currently in Cached state
        let cached_ids: Vec<PostId> = metadata
            .iter()
            .filter(|m| matches!(state_reader.get(m.id), EntityState::Cached))
            .map(|m| PostId(m.id))
            .collect();

        if cached_ids.is_empty() {
            return Vec::new();
        }

        // Query database for cached timestamps
        let cached_timestamps = self
            .cache
            .execute(|conn| {
                let repo = PostRepository::<EditContext>::new();
                repo.select_modified_gmt_by_ids(conn, &self.db_site, &cached_ids)
            })
            .unwrap_or_else(|e| {
                log::warn!(
                    "Failed to query cached timestamps for staleness check: {}",
                    e
                );
                Default::default()
            });

        // Compare timestamps and collect stale IDs
        metadata
            .iter()
            .filter_map(|m| {
                if let Some(fetched_modified) = &m.modified_gmt
                    && let Some(cached_modified) = cached_timestamps.get(&PostId(m.id))
                    && fetched_modified != cached_modified
                {
                    Some(m.id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Sync a post list using the default full sync strategy.
    ///
    /// This is a convenience method that calls `sync_list_with_strategy` with
    /// [`SyncStrategy::Full`]. See that method for full documentation.
    ///
    /// # Arguments
    /// * `key` - Metadata store key (e.g., "site_1:edit:posts:status=publish")
    /// * `endpoint_type` - The post endpoint type (Posts, Pages, or Custom)
    /// * `filter` - Filter parameters (pagination is managed internally)
    /// * `per_page` - Number of posts per page (only used for refresh)
    /// * `is_refresh` - If true, refreshes (page 1); if false, loads more (next page)
    pub async fn sync_list(
        &self,
        key: &ListKey,
        endpoint_type: &PostEndpointType,
        filter: &PostListFilter,
        per_page: u32,
        is_refresh: bool,
    ) -> Result<SyncResult, FetchError> {
        self.sync_list_with_strategy(
            key,
            endpoint_type,
            filter,
            per_page,
            is_refresh,
            SyncStrategy::Full,
        )
        .await
    }

    /// Sync a post list with explicit strategy control.
    ///
    /// Orchestrates the sync flow based on the chosen strategy:
    ///
    /// **[`SyncStrategy::MetadataOnly`]:**
    /// 1. Fetch list metadata (IDs, modified_gmt, pagination)
    /// 2. Store metadata in database
    /// 3. Detect stale posts (marks them, but doesn't fetch)
    ///
    /// **[`SyncStrategy::Full`]:**
    /// 1. All of MetadataOnly, plus:
    /// 2. Fetch missing/stale post data from the API
    ///
    /// # Arguments
    /// * `key` - Metadata store key (e.g., "site_1:edit:posts:status=publish")
    /// * `endpoint_type` - The post endpoint type (Posts, Pages, or Custom)
    /// * `filter` - Filter parameters (pagination is managed internally)
    /// * `per_page` - Number of posts per page (only used for refresh)
    /// * `is_refresh` - If true, refreshes (page 1); if false, loads more (next page)
    /// * `strategy` - Controls whether to fetch entity data or just metadata
    ///
    /// # Returns
    /// - `Ok(SyncResult)` with sync statistics
    /// - `Err(FetchError)` if network or database error occurs
    pub async fn sync_list_with_strategy(
        &self,
        key: &ListKey,
        endpoint_type: &PostEndpointType,
        filter: &PostListFilter,
        per_page: u32,
        is_refresh: bool,
        strategy: SyncStrategy,
    ) -> Result<SyncResult, FetchError> {
        // 1. Fetch and store metadata
        let metadata_result = if is_refresh {
            self.metadata_service
                .refresh(key, per_page, |page, per_page| {
                    self.fetch_posts_metadata(endpoint_type, filter, page, per_page)
                })
                .await?
        } else {
            self.metadata_service
                .load_more(key, |page, per_page| {
                    self.fetch_posts_metadata(endpoint_type, filter, page, per_page)
                })
                .await?
        };

        // 2. Detect and mark stale posts (always done - doesn't fetch)
        let stale_ids = self.find_stale_posts_by_timestamp(
            &metadata_result.metadata,
            self.state_reader_with_edit_context().as_ref(),
        );

        if !stale_ids.is_empty() {
            log::debug!(
                "Found {} stale post(s) via modified_gmt comparison",
                stale_ids.len()
            );
            // Mark them as stale in state store
            self.state_store_with_edit_context
                .set_batch(&stale_ids, EntityState::Stale);
        }

        // 3. Fetch missing/stale posts (only for Full strategy)
        let stats = match strategy {
            SyncStrategy::MetadataOnly => FetchStats {
                fetched_count: 0,
                failed_count: 0,
            },
            SyncStrategy::Full => {
                self.fetch_missing_and_stale_posts(endpoint_type, &metadata_result.metadata)
                    .await
            }
        };

        // Build result
        let total_items = self
            .metadata_service
            .get_entity_ids(key)
            .map(|ids| ids.len())
            .unwrap_or(0);

        // Get pagination info from DB
        let pagination = self.metadata_service.get_pagination(key).ok().flatten();

        let current_page = pagination.as_ref().and_then(|p| p.current_page);

        let has_more_pages = pagination.as_ref().and_then(|p| {
            current_page.and_then(|current| p.total_pages.map(|total| current < total))
        });

        Ok(SyncResult::new(
            total_items,
            stats.fetched_count,
            stats.failed_count,
            has_more_pages,
            current_page,
            metadata_result.total_pages,
        ))
    }

    /// Fetch posts that are missing or stale based on current state.
    ///
    /// Returns statistics about the fetch operation.
    pub(crate) async fn fetch_missing_and_stale_posts(
        &self,
        endpoint_type: &PostEndpointType,
        metadata: &[EntityMetadata],
    ) -> FetchStats {
        let ids_to_fetch: Vec<PostId> = metadata
            .iter()
            .filter(|m| {
                let state = self.state_store_with_edit_context.get(m.id);
                matches!(state, EntityState::Missing | EntityState::Stale)
            })
            .map(|m| PostId(m.id))
            .collect();

        let fetched_count = ids_to_fetch.len();
        let mut failed_count = 0;

        if !ids_to_fetch.is_empty() {
            // Batch into chunks
            for chunk in ids_to_fetch.chunks(BATCH_FETCH_SIZE) {
                match self.load_posts_by_ids(endpoint_type, chunk.to_vec()).await {
                    Ok(result) => {
                        // Accumulate failures reported by load_posts_by_ids
                        failed_count += result.failed_count;
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to load {} posts (IDs: {:?}): {}",
                            chunk.len(),
                            chunk,
                            e
                        );
                        // Network/DB error - all items in this chunk failed
                        failed_count += chunk.len();
                    }
                }
            }
        }

        FetchStats {
            fetched_count,
            failed_count,
        }
    }

    /// Load posts by IDs from network to cache with state tracking.
    ///
    /// Fetches posts from the API, saves them to the database, and manages entity state.
    /// Used for selective sync to load only missing or stale posts.
    ///
    /// # State Management
    ///
    /// Tracks entity lifecycle through state store:
    /// 1. Filters out IDs already `Fetching` (prevents duplicate requests)
    /// 2. Sets remaining IDs to `Fetching` before API call
    /// 3. On success: Sets fetched posts to `Cached`, missing posts to `Failed`
    /// 4. On error: Sets all requested posts to `Failed`
    ///
    /// # Arguments
    /// * `endpoint_type` - The post endpoint type (Posts, Pages, or Custom)
    /// * `ids` - Post IDs to load
    ///
    /// # Returns
    /// - `Ok(FetchByIdsResult)` with entity IDs of loaded posts and failure count
    /// - `Err(FetchError)` if network or database error occurs
    ///
    /// # Note
    /// Returns empty result without network request if `ids` is empty or all IDs are already fetching.
    pub async fn load_posts_by_ids(
        &self,
        endpoint_type: &PostEndpointType,
        ids: Vec<PostId>,
    ) -> Result<FetchByIdsResult, FetchError> {
        if ids.is_empty() {
            return Ok(FetchByIdsResult {
                entity_ids: Vec::new(),
                failed_count: 0,
            });
        }

        // Convert to raw IDs and filter out already-fetching
        let raw_ids: Vec<i64> = ids.iter().map(|id| id.0).collect();
        let fetchable = self
            .state_store_with_edit_context
            .filter_fetchable(&raw_ids);

        if fetchable.is_empty() {
            return Ok(FetchByIdsResult {
                entity_ids: Vec::new(),
                failed_count: 0,
            });
        }

        // Mark as fetching
        self.state_store_with_edit_context
            .set_batch(&fetchable, EntityState::Fetching);

        // Convert back to PostId for the API call
        let post_ids: Vec<PostId> = fetchable.iter().map(|&id| PostId(id)).collect();

        let params = PostListParams {
            include: post_ids,
            // Ensure we get all requested posts regardless of default per_page
            per_page: Some(BATCH_FETCH_SIZE as u32),
            // UI-level decision: Include posts with these statuses when fetching by ID.
            // WordPress API defaults to 'publish' only, which would filter out drafts/pending/etc.
            // We exclude 'trash' status because trashed posts aren't shown in normal post lists
            // (they have their own separate trash view in WordPress admin).
            status: vec![
                PostStatus::Publish,
                PostStatus::Draft,
                PostStatus::Pending,
                PostStatus::Private,
                PostStatus::Future,
            ],
            ..Default::default()
        };

        match self
            .api_client
            .posts()
            .list_with_edit_context(endpoint_type, &params)
            .await
        {
            Ok(response) => {
                // Upsert to database and collect entity IDs
                let entity_ids = match self.cache.execute(|conn| {
                    let repo = PostRepository::<EditContext>::new();
                    response
                        .data
                        .iter()
                        .map(|post| {
                            repo.upsert(conn, &self.db_site, post).map_err(|e| {
                                FetchError::Database {
                                    err_message: e.to_string(),
                                }
                            })
                        })
                        .collect::<Result<Vec<EntityId>, FetchError>>()
                }) {
                    Ok(ids) => ids,
                    Err(e) => {
                        // Database upsert failed - mark all as failed to avoid stuck Fetching state
                        self.state_store_with_edit_context
                            .set_batch(&fetchable, EntityState::failed(e.to_string()));
                        return Err(e);
                    }
                };

                // Mark successfully fetched posts as Cached
                let fetched_ids: Vec<i64> = response.data.iter().map(|p| p.id.0).collect();
                self.state_store_with_edit_context
                    .set_batch(&fetched_ids, EntityState::Cached);

                // Mark posts that were requested but not returned as Failed
                let fetched_set: HashSet<i64> = fetched_ids.iter().copied().collect();
                let failed_ids: Vec<i64> = fetchable
                    .iter()
                    .filter(|id| !fetched_set.contains(id))
                    .copied()
                    .collect();
                let failed_count = failed_ids.len();
                if !failed_ids.is_empty() {
                    self.state_store_with_edit_context
                        .set_batch(&failed_ids, EntityState::failed("Not found"));
                }

                Ok(FetchByIdsResult {
                    entity_ids,
                    failed_count,
                })
            }
            Err(e) => {
                // Network/API error - mark all as failed
                self.state_store_with_edit_context
                    .set_batch(&fetchable, EntityState::failed(e.to_string()));
                Err(e.into())
            }
        }
    }

    /// Get read-only access to the entity state store for edit context.
    ///
    /// Used by `MetadataCollection` to read entity states without
    /// being able to modify them.
    pub fn state_reader_with_edit_context(&self) -> Arc<dyn EntityStateReader> {
        self.state_store_with_edit_context.clone()
    }

    /// Get read-only access to the persistent metadata service.
    ///
    /// Returns a reader backed by the database, so list metadata persists
    /// across app restarts. Use this for production collections.
    pub fn persistent_metadata_reader(&self) -> Arc<MetadataService> {
        self.metadata_service.clone()
    }

    /// Read posts by IDs from the database cache.
    ///
    /// Returns full entity data for all requested IDs that exist in the cache.
    /// Posts not in the cache are silently omitted from the result.
    ///
    /// # Arguments
    /// * `ids` - Post IDs to load
    ///
    /// # Returns
    /// - `Ok(Vec<FullEntity>)` with posts found in cache
    /// - `Err` if database error occurs
    pub fn read_posts_by_ids_from_db(
        &self,
        ids: &[i64],
    ) -> Result<Vec<FullEntity<AnyPostWithEditContext>>, wp_mobile_cache::SqliteDbError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let repo = PostRepository::<EditContext>::new();

        self.cache.execute(|connection| {
            ids.iter()
                .map(|&id| repo.select_by_post_id(connection, &self.db_site, PostId(id)))
                .collect::<Result<Vec<_>, _>>()
                .map(|posts| {
                    posts
                        .into_iter()
                        .flatten()
                        .map(|db_post| FullEntity::new(db_post.entity_id, db_post.data.post))
                        .collect()
                })
        })
    }
}

#[uniffi::export]
impl PostService {
    /// Get an entity handle using an EntityId
    ///
    /// Returns an entity that can be used to read post data with full edit context.
    /// The entity is lightweight - it doesn't fetch data until you call load_data() on it.
    ///
    /// The EntityId should come from repository results (e.g., select_by_post_id).
    pub fn get_entity_with_edit_context(
        &self,
        entity_id: EntityId,
    ) -> EntityAnyPostWithEditContext {
        let cache = self.cache.clone();

        Entity::<AnyPostWithEditContext>::new(
            entity_id,
            Box::new(move || {
                let repo = PostRepository::<EditContext>::new();

                cache
                    .execute(|connection| repo.select_by_entity_id(connection, &entity_id))
                    .map(|opt| {
                        opt.map(|db_post_full_entity| {
                            FullEntity::new(
                                db_post_full_entity.entity_id,
                                db_post_full_entity.data.post,
                            )
                        })
                    })
            }),
        )
        .into()
    }

    /// Get the total count of posts for this site
    ///
    /// Returns the number of posts stored in the cache for this site.
    pub fn count_edit_context(&self) -> Result<i64, wp_mobile_cache::SqliteDbError> {
        let repo = PostRepository::<EditContext>::new();
        self.cache
            .execute(|connection| repo.count(connection, &self.db_site))
    }

    /// Delete a post by its EntityId
    ///
    /// Returns the number of rows deleted (0 or 1).
    /// Automatically deletes associated term relationships.
    ///
    /// # Arguments
    /// * `entity_id` - The EntityId of the post to delete
    ///
    /// # Returns
    /// - `Ok(1)` if the post was deleted
    /// - `Ok(0)` if the post doesn't exist
    /// - `Err` if there was a database error
    pub fn delete_by_entity_id(
        &self,
        entity_id: &EntityId,
    ) -> Result<u64, wp_mobile_cache::SqliteDbError> {
        let repo = PostRepository::<EditContext>::new();
        self.cache.execute(|connection| {
            repo.delete_by_entity_id(connection, entity_id)
                .map(|n| n as u64)
        })
    }

    /// Delete a post by its WordPress post ID
    ///
    /// Returns the number of rows deleted (0 or 1).
    /// Automatically deletes associated term relationships.
    ///
    /// # Arguments
    /// * `post_id` - The WordPress post ID to delete
    ///
    /// # Returns
    /// - `Ok(1)` if the post was deleted
    /// - `Ok(0)` if the post doesn't exist
    /// - `Err` if there was a database error
    pub fn delete_by_post_id(
        &self,
        post_id: wp_api::posts::PostId,
    ) -> Result<u64, wp_mobile_cache::SqliteDbError> {
        let repo = PostRepository::<EditContext>::new();
        self.cache.execute(|connection| {
            repo.delete_by_post_id(connection, &self.db_site, post_id)
                .map(|n| n as u64)
        })
    }

    /// Create a filtered post collection with edit context
    ///
    /// Returns a collection that:
    /// - Filters posts based on the provided filter criteria
    /// - Supports network fetching via fetch_page()
    /// - Monitors database changes and provides load_data() for cache access
    ///
    /// # Arguments
    /// * `filter` - Filter criteria for posts (status, etc.)
    ///
    /// # Example (Kotlin)
    /// ```kotlin
    /// val filter = AnyPostFilter(status = PostStatus.DRAFT)
    /// val collection = postService.createPostCollectionWithEditContext(filter)
    ///
    /// // Fetch from network
    /// val result = collection.fetchPage(1u, 10u)
    ///
    /// // Load from cache
    /// val posts = collection.loadData()
    /// ```
    pub fn create_post_collection_with_edit_context(
        self: &Arc<Self>,
        filter: AnyPostFilter,
    ) -> PostCollectionWithEditContext {
        let cache = self.cache.clone();
        let db_site = *self.db_site;
        let filter_clone = filter.clone();

        // Create StatelessCollection with filtering
        let stateless_collection = StatelessCollection::new(
            vec![DbTable::PostsEditContext, DbTable::TermRelationships],
            Box::new(move || {
                let repo = PostRepository::<EditContext>::new();
                cache.execute(|connection| {
                    repo.select_by_filter(connection, &db_site, filter_clone.status.as_ref())
                        .map(|posts| {
                            posts
                                .into_iter()
                                .map(|db_post_full_entity| {
                                    FullEntity::new(
                                        db_post_full_entity.entity_id,
                                        db_post_full_entity.data.post,
                                    )
                                })
                                .collect()
                        })
                })
            }),
        );

        PostCollection::new(filter, stateless_collection, self.clone()).into()
    }

    /// Create a metadata-first post collection with edit context
    ///
    /// Returns a collection that uses a two-phase sync strategy:
    /// 1. Fetch lightweight metadata (id + modified_gmt) to define list structure
    /// 2. Selectively fetch full data for missing or stale items
    ///
    /// Unlike `create_post_collection_with_edit_context` which fetches full data,
    /// this collection shows cached items immediately and fetches only what's needed.
    ///
    /// # Arguments
    /// * `endpoint_type` - The post endpoint type (Posts, Pages, or Custom)
    /// * `filter` - Filter parameters (status, author, categories, etc.)
    /// * `per_page` - Number of items per page
    ///
    /// # Example (Kotlin)
    /// ```kotlin
    /// val filter = PostListFilter(status = listOf(PostStatus.DRAFT))
    /// val collection = postService.createPostMetadataCollectionWithEditContext(
    ///     PostEndpointType.POSTS,
    ///     filter,
    ///     20u
    /// )
    ///
    /// // Initial load - fetches metadata, then syncs missing items
    /// collection.refresh()
    ///
    /// // Get items with states and data
    /// val items = collection.loadItems()
    /// ```
    pub fn create_post_metadata_collection_with_edit_context(
        self: &Arc<Self>,
        endpoint_type: PostEndpointType,
        filter: PostListFilter,
        per_page: u32,
    ) -> PostMetadataCollectionWithEditContext {
        // Generate cache key from filter
        let cache_key = post_list_filter_cache_key(&filter);
        let endpoint_key = endpoint_type_cache_key(&endpoint_type);
        let key: ListKey = format!(
            "site_{:?}:edit:{}:{}",
            self.db_site.row_id, endpoint_key, cache_key
        )
        .into();

        let core = MetadataCollectionCore::new(
            key,
            self.persistent_metadata_reader(),
            self.state_reader_with_edit_context(),
            vec![
                DbTable::PostsEditContext,
                DbTable::TermRelationships,
                DbTable::ListMetadataItems,
            ],
            per_page,
        );

        PostMetadataCollectionWithEditContext::new(core, self.clone(), endpoint_type, filter)
    }

    /// Get a collection of all posts with edit context for this site.
    ///
    /// Returns a collection that can be used to observe all posts for this site.
    /// The collection monitors both the posts table and term relationships table -
    /// any insert, update, or delete to either table will trigger observers.
    ///
    /// Unlike individual entities, the collection re-queries all posts when any
    /// relevant change occurs.
    pub fn get_all_posts_with_edit_context(&self) -> AllAnyPostWithEditContextCollection {
        let cache = self.cache.clone();
        let db_site = *self.db_site;

        StatelessCollection::new(
            vec![
                wp_mobile_cache::DbTable::PostsEditContext,
                wp_mobile_cache::DbTable::TermRelationships,
            ],
            Box::new(move || {
                let repo = PostRepository::<EditContext>::new();
                cache.execute(|connection| {
                    repo.select_all(connection, &db_site).map(|posts| {
                        posts
                            .into_iter()
                            .map(|db_post_full_entity| {
                                FullEntity::new(
                                    db_post_full_entity.entity_id,
                                    db_post_full_entity.data.post,
                                )
                            })
                            .collect()
                    })
                })
            }),
        )
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::mock_api_client;
    use rstest::*;
    use rusqlite::Connection;
    use wp_api::posts::PostId;
    use wp_api::prelude::*;
    use wp_mobile_cache::{
        HookAction, MigrationManager, UpdateHook, WpApiCache,
        db_types::self_hosted_site::SelfHostedSite,
        repository::{posts::PostRepository, sites::SiteRepository},
        test_fixtures::posts::PostBuilder,
    };

    #[rstest]
    fn test_get_entity_load_data_returns_cached_post(post_service_ctx: PostServiceTestContext) {
        // Setup: Insert test post into cache
        let test_post = insert_test_post(&post_service_ctx);

        // Test: Get EntityId from repository, then create entity
        let entity_id = post_service_ctx
            .cache
            .execute(|conn| {
                let repo = PostRepository::<EditContext>::new();
                repo.select_by_post_id(conn, &post_service_ctx.db_site, test_post.id)
                    .map(|opt| opt.map(|full_entity| *full_entity.entity_id))
            })
            .expect("Database read should succeed")
            .expect("Post should exist");

        let entity = post_service_ctx
            .post_service
            .get_entity_with_edit_context(entity_id);
        // Use the internal Entity's sync load_data for testing
        let result = entity.0.load_data().expect("Database read should succeed");

        // Assert: Post was found and matches what we inserted
        let full_entity = result.expect("Post should be found in cache");
        test_post.assert_matches(&full_entity.data);
    }

    #[rstest]
    fn test_entity_is_relevant_update_matches_correct_updates(
        post_service_ctx: PostServiceTestContext,
    ) {
        // Setup: Insert test post
        let test_post = insert_test_post(&post_service_ctx);

        // Get EntityId from repository
        let entity_id = post_service_ctx
            .cache
            .execute(|conn| {
                let repo = PostRepository::<EditContext>::new();
                repo.select_by_post_id(conn, &post_service_ctx.db_site, test_post.id)
                    .map(|opt| opt.map(|full_entity| *full_entity.entity_id))
            })
            .expect("Database read should succeed")
            .expect("Post should exist");

        let entity = post_service_ctx
            .post_service
            .get_entity_with_edit_context(entity_id);

        // Get the table and rowid from the entity_id
        let table = entity_id.table;
        let rowid = entity_id.rowid.0;

        // Test: Create UpdateHook that matches this entity
        let matching_hook = UpdateHook {
            action: HookAction::Update,
            db_name: "main".to_string(),
            table,
            row_id: rowid,
        };

        // Assert: Entity should recognize this update as relevant
        assert!(
            entity.0.is_relevant_update(&matching_hook),
            "Entity should match updates with same table and rowid"
        );

        // Test: Create UpdateHook with different table
        let wrong_table_hook = UpdateHook {
            action: HookAction::Update,
            db_name: "main".to_string(),
            table: wp_mobile_cache::DbTable::PostsViewContext, // Different table
            row_id: rowid,
        };

        // Assert: Entity should not match updates from different table
        assert!(
            !entity.0.is_relevant_update(&wrong_table_hook),
            "Entity should not match updates from different table"
        );

        // Test: Create UpdateHook with different rowid
        let wrong_rowid_hook = UpdateHook {
            action: HookAction::Update,
            db_name: "main".to_string(),
            table,
            row_id: rowid + 1,
        };

        // Assert: Entity should not match updates for different row
        assert!(
            !entity.0.is_relevant_update(&wrong_rowid_hook),
            "Entity should not match updates for different rowid"
        );
    }

    /// Test helper that encapsulates a test post with its assertion logic
    struct TestPost {
        id: PostId,
        title: String,
        slug: String,
    }

    impl TestPost {
        /// Assert that a retrieved post matches the expected values
        fn assert_matches(&self, post: &AnyPostWithEditContext) {
            assert_eq!(post.id, self.id);
            assert_eq!(post.title.rendered, self.title);
            assert_eq!(post.slug, self.slug);
        }
    }

    /// Helper function to insert a test post into the cache
    ///
    /// Creates a test post with predefined values and inserts it into the database.
    /// Returns a TestPost that can be used to assert the retrieved data matches what was inserted.
    /// This common setup is used by multiple tests to showcase similarities between
    /// direct database reads and entity-based reads.
    fn insert_test_post(ctx: &PostServiceTestContext) -> TestPost {
        let test_post = TestPost {
            id: PostId(42),
            title: "Test Post".to_string(),
            slug: "test-post".to_string(),
        };

        let post = PostBuilder::minimal()
            .with_id(test_post.id.0)
            .with_title(&test_post.title)
            .with_slug(&test_post.slug)
            .build();

        ctx.cache
            .execute(|conn| {
                let post_repo = PostRepository::<EditContext>::new();
                post_repo.upsert(conn, &ctx.db_site, &post)
            })
            .expect("Post insert should succeed");

        test_post
    }

    /// Test context bundling PostService with database and site setup
    pub struct PostServiceTestContext {
        pub post_service: PostService,
        pub db_site: Arc<DbSite>,
        pub cache: Arc<WpApiCache>,
    }

    #[rstest]
    fn test_delete_by_entity_id(post_service_ctx: PostServiceTestContext) {
        // Setup: Insert test post
        let test_post = insert_test_post(&post_service_ctx);
        let entity_id = post_service_ctx
            .cache
            .execute(|conn| {
                let repo = PostRepository::<EditContext>::new();
                repo.select_by_post_id(conn, &post_service_ctx.db_site, test_post.id)
                    .map(|opt| opt.map(|full_entity| *full_entity.entity_id))
            })
            .expect("Database read should succeed")
            .expect("Post should exist");

        // Test: Delete by entity_id
        let deleted = post_service_ctx
            .post_service
            .delete_by_entity_id(&entity_id)
            .expect("Delete should succeed");

        // Assert: Post was deleted
        assert_eq!(deleted, 1, "Should delete 1 post");

        // Verify post no longer exists
        let result = post_service_ctx.cache.execute(|conn| {
            let repo = PostRepository::<EditContext>::new();
            repo.select_by_entity_id(conn, &entity_id)
        });
        assert!(
            result.unwrap().is_none(),
            "Post should not exist after deletion"
        );
    }

    #[rstest]
    fn test_delete_by_post_id(post_service_ctx: PostServiceTestContext) {
        // Setup: Insert test post
        let test_post = insert_test_post(&post_service_ctx);

        // Test: Delete by post_id
        let deleted = post_service_ctx
            .post_service
            .delete_by_post_id(test_post.id)
            .expect("Delete should succeed");

        // Assert: Post was deleted
        assert_eq!(deleted, 1, "Should delete 1 post");

        // Verify post no longer exists
        let result = post_service_ctx.cache.execute(|conn| {
            let repo = PostRepository::<EditContext>::new();
            repo.select_by_post_id(conn, &post_service_ctx.db_site, test_post.id)
        });
        assert!(
            result.unwrap().is_none(),
            "Post should not exist after deletion"
        );
    }

    #[rstest]
    fn test_delete_by_entity_id_non_existent_returns_zero(
        post_service_ctx: PostServiceTestContext,
    ) {
        // Setup: Insert a post and get its entity_id
        let test_post = insert_test_post(&post_service_ctx);
        let entity_id = post_service_ctx
            .cache
            .execute(|conn| {
                let repo = PostRepository::<EditContext>::new();
                repo.select_by_post_id(conn, &post_service_ctx.db_site, test_post.id)
                    .map(|opt| opt.map(|full_entity| *full_entity.entity_id))
            })
            .expect("Database read should succeed")
            .expect("Post should exist");

        // Setup: Delete the post via service
        post_service_ctx
            .post_service
            .delete_by_entity_id(&entity_id)
            .expect("First delete should succeed");

        // Test: Try to delete again with the same entity_id (now non-existent)
        let deleted = post_service_ctx
            .post_service
            .delete_by_entity_id(&entity_id)
            .expect("Delete should not error");

        // Assert: Should return 0
        assert_eq!(deleted, 0, "Should return 0 for non-existent post");
    }

    #[rstest]
    fn test_delete_by_post_id_non_existent_returns_zero(post_service_ctx: PostServiceTestContext) {
        // Test: Delete non-existent post
        let deleted = post_service_ctx
            .post_service
            .delete_by_post_id(PostId(99999))
            .expect("Delete should not error");

        // Assert: Should return 0
        assert_eq!(deleted, 0, "Should return 0 for non-existent post");
    }

    // ============================================================
    // State management and sync tests
    // ============================================================

    #[rstest]
    fn test_find_stale_posts_by_timestamp_ignores_posts_without_modified_gmt_in_metadata(
        post_service_ctx: PostServiceTestContext,
    ) {
        use crate::sync::EntityMetadata;

        // Setup: Insert a post and mark it as Cached
        let test_post = insert_test_post(&post_service_ctx);
        post_service_ctx
            .post_service
            .state_store_with_edit_context
            .set(test_post.id.0, crate::sync::EntityState::Cached);

        // Test: Metadata without modified_gmt (None)
        let metadata = vec![EntityMetadata::new(test_post.id.0, None, None, None)];

        let stale_ids = post_service_ctx.post_service.find_stale_posts_by_timestamp(
            &metadata,
            post_service_ctx
                .post_service
                .state_reader_with_edit_context()
                .as_ref(),
        );

        // Assert: No posts should be identified as stale
        assert!(
            stale_ids.is_empty(),
            "Posts without modified_gmt in metadata should not be marked as stale"
        );

        // Verify state hasn't changed
        let state = post_service_ctx
            .post_service
            .state_store_with_edit_context
            .get(test_post.id.0);
        assert!(
            matches!(state, crate::sync::EntityState::Cached),
            "State should remain Cached"
        );
    }

    #[rstest]
    fn test_find_stale_posts_by_timestamp_ignores_non_cached_posts(
        post_service_ctx: PostServiceTestContext,
    ) {
        use crate::sync::EntityMetadata;

        // Setup: Insert a post but don't mark it as Cached (it's Missing by default)
        let test_post = insert_test_post(&post_service_ctx);
        let modified = "2024-01-01T12:00:00Z"
            .parse::<wp_api::prelude::WpGmtDateTime>()
            .expect("Parse should succeed");

        // Test: Metadata with modified_gmt
        let metadata = vec![EntityMetadata::new(
            test_post.id.0,
            Some(modified),
            None,
            None,
        )];

        let stale_ids = post_service_ctx.post_service.find_stale_posts_by_timestamp(
            &metadata,
            post_service_ctx
                .post_service
                .state_reader_with_edit_context()
                .as_ref(),
        );

        // Assert: No posts should be identified as stale (only Cached posts are checked)
        assert!(
            stale_ids.is_empty(),
            "Non-cached posts should not be identified as stale"
        );

        // Verify state hasn't changed
        let state = post_service_ctx
            .post_service
            .state_store_with_edit_context
            .get(test_post.id.0);
        assert!(
            matches!(state, crate::sync::EntityState::Missing),
            "State should remain Missing"
        );
    }

    // State transition tests
    #[rstest]
    #[tokio::test]
    async fn test_load_posts_by_ids_marks_all_as_failed_on_network_error() {
        use crate::testing::{EmptyAppNotifier, MockExecutor};
        use wp_api::request::endpoint::posts_endpoint::PostEndpointType;

        // Setup: Create service with mock executor that returns network error
        let mock_executor = Arc::new(MockExecutor::with_execute_fn(|_| {
            Err(RequestExecutionError::RequestExecutionFailed {
                status_code: None,
                redirects: None,
                reason: RequestExecutionErrorReason::GenericError {
                    error_message: "Network timeout".to_string(),
                },
            })
        }));

        let api_root_url =
            Arc::new(ParsedUrl::parse("https://test.local/wp-json").expect("Parse URL"));
        let api_client = Arc::new(WpApiClient::new(
            Arc::new(WpOrgSiteApiUrlResolver::new(api_root_url)),
            WpApiClientDelegate {
                auth_provider: Arc::new(WpAuthenticationProvider::none()),
                request_executor: mock_executor,
                middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
                app_notifier: Arc::new(EmptyAppNotifier),
            },
        ));

        let mut conn = Connection::open_in_memory().expect("Create in-memory database");
        let mut migration_manager = MigrationManager::new(&conn).expect("Create migration manager");
        migration_manager
            .perform_migrations()
            .expect("Migrations succeed");

        let site_repo = SiteRepository;
        let self_hosted_site = SelfHostedSite {
            url: "https://test.local".to_string(),
            api_root: "https://test.local/wp-json".to_string(),
        };
        let db_site = site_repo
            .upsert_self_hosted_site(&mut conn, &self_hosted_site)
            .expect("Site creation")
            .db_site;

        let cache = Arc::new(WpApiCache::from(conn));
        let db_site_arc = Arc::new(db_site);
        let service = PostService::new(api_client, db_site_arc, cache);

        // Test: Try to load posts
        let result = service
            .load_posts_by_ids(&PostEndpointType::Posts, vec![PostId(1), PostId(2)])
            .await;

        // Assert: Should return error
        assert!(result.is_err(), "Network error should return Err");

        // Assert: Posts should be marked as Failed
        let state1 = service.state_store_with_edit_context.get(1);
        let state2 = service.state_store_with_edit_context.get(2);
        assert!(
            matches!(state1, crate::sync::EntityState::Failed { .. }),
            "Post 1 should be marked as Failed on network error"
        );
        assert!(
            matches!(state2, crate::sync::EntityState::Failed { .. }),
            "Post 2 should be marked as Failed on network error"
        );
    }

    /// rstest fixture providing a PostService with in-memory database
    ///
    /// Sets up an in-memory SQLite database with migrations, creates a test site,
    /// and returns a PostService instance ready for testing.
    ///
    /// # Example
    ///
    /// ```rust
    /// #[rstest]
    /// fn test_something(post_service_ctx: PostServiceTestContext) {
    ///     let result = post_service_ctx.post_service.read_post_from_db(PostId(1));
    ///     // ...
    /// }
    /// ```
    #[fixture]
    fn post_service_ctx(mock_api_client: Arc<WpApiClient>) -> PostServiceTestContext {
        // Setup: Create in-memory database with migrations
        let mut conn = Connection::open_in_memory().expect("Failed to create in-memory database");
        let mut migration_manager =
            MigrationManager::new(&conn).expect("Failed to create migration manager");
        migration_manager
            .perform_migrations()
            .expect("Migrations should succeed");

        // Setup: Create test site
        let site_repo = SiteRepository;
        let self_hosted_site = SelfHostedSite {
            url: "https://test.local".to_string(),
            api_root: "https://test.local/wp-json".to_string(),
        };
        let db_site = site_repo
            .upsert_self_hosted_site(&mut conn, &self_hosted_site)
            .expect("Site creation should succeed")
            .db_site;

        // Setup: Create PostService with cache
        let cache = Arc::new(WpApiCache::from(conn));
        let db_site_arc = Arc::new(db_site);
        let post_service = PostService::new(mock_api_client, db_site_arc.clone(), cache.clone());

        PostServiceTestContext {
            post_service,
            db_site: db_site_arc,
            cache,
        }
    }
}
