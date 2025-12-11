use crate::{
    AllAnyPostWithEditContextCollection, EntityAnyPostWithEditContext,
    PostCollectionWithEditContext,
    collection::{
        FetchError, FetchResult, PostMetadataCollectionWithEditContext, StatelessCollection,
        post_collection::PostCollection,
    },
    filters::AnyPostFilter,
    service::metadata::MetadataService,
    sync::{
        EntityMetadata, EntityState, EntityStateReader, EntityStateStore, MetadataCollection,
        MetadataFetchResult, PersistentPostMetadataFetcherWithEditContext, SyncResult,
    },
};
use std::sync::Arc;
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
    repository::posts::PostRepository,
};

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

    /// Fetch posts from network and save to cache
    ///
    /// This is the core networking primitive. It:
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
    /// - `Ok(FetchResult)` with entity IDs of fetched posts
    /// - `Err(FetchError)` if network or database error occurs
    ///
    /// # Database Updates
    /// Successful fetch triggers database update hooks, which notify
    /// any observers watching the relevant tables.
    ///
    /// # Note
    /// This is an async function because network operations are async.
    /// Platform-specific wrappers (Kotlin/Swift) will need to handle
    /// the async bridge.
    pub async fn fetch_posts_page(
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
                .collect::<Result<Vec<_>, _>>()
        })?;

        Ok(FetchResult {
            entity_ids,
            total_items: response.header_map.wp_total().map(|n| n as i64),
            total_pages: response.header_map.wp_total_pages(),
            current_page: page,
        })
    }

    /// Fetch only metadata (id + modified_gmt) for a page of posts.
    ///
    /// This is a lightweight fetch that returns just enough information to:
    /// 1. Define list structure (order and IDs)
    /// 2. Determine which posts need full fetching (missing or stale)
    ///
    /// Unlike `fetch_posts_page`, this does NOT upsert to the database.
    /// The metadata is used transiently to drive selective sync.
    ///
    /// # Arguments
    /// * `filter` - Post filter criteria
    /// * `page` - Page number to fetch (1-indexed)
    /// * `per_page` - Number of posts per page
    ///
    /// # Returns
    /// - `Ok(MetadataFetchResult)` with post IDs and modification times
    /// - `Err(FetchError)` if network error occurs
    pub async fn fetch_posts_metadata(
        &self,
        filter: &AnyPostFilter,
        page: u32,
        per_page: u32,
    ) -> Result<MetadataFetchResult, FetchError> {
        let mut params = filter.to_list_params();
        params.page = Some(page);
        params.per_page = Some(per_page);

        let response = self
            .api_client
            .posts()
            .filter_list_with_edit_context(
                &PostEndpointType::Posts,
                &params,
                &[
                    SparseAnyPostFieldWithEditContext::Id,
                    SparseAnyPostFieldWithEditContext::ModifiedGmt,
                ],
            )
            .await?;

        // Map sparse posts to EntityMetadata, filtering out any with missing id
        let metadata: Vec<EntityMetadata> = response
            .data
            .into_iter()
            .filter_map(|sparse| Some(EntityMetadata::new(sparse.id?.0, sparse.modified_gmt)))
            .collect();

        Ok(MetadataFetchResult::new(
            metadata,
            response.header_map.wp_total().map(|n| n as i64),
            response.header_map.wp_total_pages(),
            page,
        ))
    }

    /// Fetch metadata and store it in the persistent database.
    ///
    /// Stores metadata to `MetadataService` (database-backed) so list structure
    /// persists across app restarts.
    ///
    /// # Arguments
    /// * `kv_key` - Key for the metadata store (e.g., "site_1:posts:publish")
    /// * `filter` - Post filter criteria
    /// * `page` - Page number to fetch (1-indexed)
    /// * `per_page` - Number of posts per page
    /// * `is_first_page` - If true, replaces metadata; if false, appends
    ///
    /// # Returns
    /// - `Ok(MetadataFetchResult)` with post IDs and modification times
    /// - `Err(FetchError)` if network or database error occurs
    pub async fn fetch_and_store_metadata_persistent(
        &self,
        kv_key: &str,
        filter: &AnyPostFilter,
        page: u32,
        per_page: u32,
        is_first_page: bool,
    ) -> Result<MetadataFetchResult, FetchError> {
        println!(
            "[fetch_and_store_metadata_persistent] Starting: key={}, page={}, is_first_page={}",
            kv_key, page, is_first_page
        );

        // Update state to fetching (this creates the list if needed)
        if is_first_page {
            println!("[fetch_and_store_metadata_persistent] Calling begin_refresh...");
            if let Err(e) = self.metadata_service.begin_refresh(kv_key) {
                println!("[fetch_and_store_metadata_persistent] begin_refresh failed: {}", e);
                return Err(FetchError::Database {
                    err_message: e.to_string(),
                });
            }
            println!("[fetch_and_store_metadata_persistent] begin_refresh succeeded");
        } else {
            println!("[fetch_and_store_metadata_persistent] Calling begin_fetch_next_page...");
            match self.metadata_service.begin_fetch_next_page(kv_key) {
                Ok(Some(_)) => println!("[fetch_and_store_metadata_persistent] begin_fetch_next_page succeeded"),
                Ok(None) => {
                    // No pages to fetch - either no pages loaded yet or already at last page
                    // This shouldn't happen if the caller checked properly, but handle it gracefully
                    println!("[fetch_and_store_metadata_persistent] begin_fetch_next_page returned None - need refresh first or at last page");
                    return Err(FetchError::Database {
                        err_message: "Cannot load next page: no pages loaded yet or already at last page. Try refresh first.".to_string(),
                    });
                }
                Err(e) => {
                    println!("[fetch_and_store_metadata_persistent] begin_fetch_next_page failed: {}", e);
                    return Err(FetchError::Database {
                        err_message: e.to_string(),
                    });
                }
            }
        }

        // Fetch metadata from network
        println!("[fetch_and_store_metadata_persistent] Fetching from network...");
        let result = match self.fetch_posts_metadata(filter, page, per_page).await {
            Ok(result) => {
                println!(
                    "[fetch_and_store_metadata_persistent] Network fetch succeeded: {} items",
                    result.metadata.len()
                );
                result
            }
            Err(e) => {
                println!("[fetch_and_store_metadata_persistent] Network fetch failed: {}", e);
                // Mark sync as failed
                let _ = self
                    .metadata_service
                    .complete_sync_with_error(kv_key, &e.to_string());
                return Err(e);
            }
        };

        // Store metadata to database
        println!("[fetch_and_store_metadata_persistent] Storing metadata to database...");
        let store_result = if is_first_page {
            self.metadata_service.set_items(kv_key, &result.metadata)
        } else {
            self.metadata_service.append_items(kv_key, &result.metadata)
        };

        if let Err(e) = store_result {
            println!("[fetch_and_store_metadata_persistent] Store metadata failed: {}", e);
            let _ = self
                .metadata_service
                .complete_sync_with_error(kv_key, &e.to_string());
            return Err(FetchError::Database {
                err_message: e.to_string(),
            });
        }
        println!("[fetch_and_store_metadata_persistent] Store metadata succeeded");

        // Update pagination info
        println!("[fetch_and_store_metadata_persistent] Updating pagination...");
        if let Err(e) = self.metadata_service.update_pagination(
            kv_key,
            result.total_pages.map(|p| p as i64),
            result.total_items,
            page as i64,
            per_page as i64,
        ) {
            println!("[fetch_and_store_metadata_persistent] Update pagination failed: {}", e);
            let _ = self
                .metadata_service
                .complete_sync_with_error(kv_key, &e.to_string());
            return Err(FetchError::Database {
                err_message: e.to_string(),
            });
        }
        println!("[fetch_and_store_metadata_persistent] Update pagination succeeded");

        // Detect stale posts by comparing modified_gmt
        self.detect_and_mark_stale_posts(&result.metadata);

        // Mark sync as complete
        println!("[fetch_and_store_metadata_persistent] Calling complete_sync...");
        if let Err(e) = self.metadata_service.complete_sync(kv_key) {
            println!("[fetch_and_store_metadata_persistent] complete_sync failed: {}", e);
            return Err(FetchError::Database {
                err_message: e.to_string(),
            });
        }
        println!("[fetch_and_store_metadata_persistent] complete_sync succeeded, returning result");

        Ok(result)
    }

    /// Compare fetched metadata against cached posts and mark stale ones.
    ///
    /// For each post that is currently `Cached`, compares the fetched `modified_gmt`
    /// against the database value. If they differ, the post is marked as `Stale`.
    fn detect_and_mark_stale_posts(&self, metadata: &[EntityMetadata]) {
        // Get IDs of posts that are currently Cached (candidates for staleness check)
        let cached_ids: Vec<i64> = metadata
            .iter()
            .filter(|m| {
                matches!(
                    self.state_store_with_edit_context.get(m.id),
                    EntityState::Cached
                )
            })
            .map(|m| m.id)
            .collect();

        if cached_ids.is_empty() {
            return;
        }

        // Query database for cached modified_gmt values
        let cached_timestamps = self
            .cache
            .execute(|conn| {
                let repo = PostRepository::<EditContext>::new();
                repo.select_modified_gmt_by_ids(conn, &self.db_site, &cached_ids)
            })
            .unwrap_or_default();

        // Compare and mark stale
        let mut stale_count = 0;
        for m in metadata.iter().filter(|m| cached_ids.contains(&m.id)) {
            if let Some(fetched_modified) = &m.modified_gmt
                && let Some(cached_modified) = cached_timestamps.get(&m.id)
                && fetched_modified != cached_modified
            {
                self.state_store_with_edit_context
                    .set(m.id, EntityState::Stale);
                stale_count += 1;
            }
        }

        if stale_count > 0 {
            println!(
                "[PostService] Detected {} stale post(s) via modified_gmt comparison",
                stale_count
            );
        }
    }

    /// Sync a post list using persistent metadata storage.
    ///
    /// This method orchestrates the full sync flow:
    /// 1. Updates state via MetadataService (FetchingFirstPage or FetchingNextPage)
    /// 2. Fetches metadata from API
    /// 3. Stores metadata in database via MetadataService
    /// 4. Detects stale posts by comparing modified_gmt
    /// 5. Fetches missing/stale posts
    /// 6. Updates pagination info
    /// 7. Sets state back to Idle (or Error on failure)
    ///
    /// # Arguments
    /// * `key` - Metadata store key (e.g., "site_1:edit:posts:publish")
    /// * `filter` - Post filter criteria
    /// * `page` - Page number to fetch (1-indexed)
    /// * `per_page` - Number of posts per page
    /// * `is_refresh` - If true, replaces metadata; if false, appends
    ///
    /// # Returns
    /// - `Ok(SyncResult)` with sync statistics
    /// - `Err(FetchError)` if network or database error occurs
    pub async fn sync_post_list(
        &self,
        key: &str,
        filter: &AnyPostFilter,
        page: u32,
        per_page: u32,
        is_refresh: bool,
    ) -> Result<SyncResult, FetchError> {
        use crate::service::WpServiceError;
        use wp_mobile_cache::list_metadata::ListState;

        // 1. Update state to fetching
        let state = if is_refresh {
            ListState::FetchingFirstPage
        } else {
            ListState::FetchingNextPage
        };

        self.metadata_service
            .set_state(key, state, None)
            .map_err(|e| match e {
                WpServiceError::DatabaseError { err_message } => FetchError::Database { err_message },
                WpServiceError::SiteNotFound => FetchError::Database {
                    err_message: "Site not found".to_string(),
                },
            })?;

        // 2. Fetch metadata from API
        let metadata_result = match self.fetch_posts_metadata(filter, page, per_page).await {
            Ok(result) => result,
            Err(e) => {
                // Update state to error
                let _ = self
                    .metadata_service
                    .complete_sync_with_error(key, &e.to_string());
                return Err(e);
            }
        };

        // 3. Store metadata in database
        let store_result = if is_refresh {
            self.metadata_service.set_items(key, &metadata_result.metadata)
        } else {
            self.metadata_service.append_items(key, &metadata_result.metadata)
        };

        if let Err(e) = store_result {
            let _ = self.metadata_service.complete_sync_with_error(key, &e.to_string());
            return Err(FetchError::Database {
                err_message: e.to_string(),
            });
        }

        // 4. Detect stale posts
        self.detect_and_mark_stale_posts(&metadata_result.metadata);

        // 5. Fetch missing/stale posts
        let ids_to_fetch: Vec<PostId> = metadata_result
            .metadata
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
            // Batch into chunks of 100
            for chunk in ids_to_fetch.chunks(100) {
                if let Err(_e) = self.fetch_posts_by_ids(chunk.to_vec()).await {
                    // Count failures - items not marked as Cached are considered failed
                    failed_count += chunk
                        .iter()
                        .filter(|id| {
                            !matches!(
                                self.state_store_with_edit_context.get(id.0),
                                EntityState::Cached
                            )
                        })
                        .count();
                }
            }
        }

        // 6. Update pagination info
        let _ = self.metadata_service.update_pagination(
            key,
            metadata_result.total_pages.map(|p| p as i64),
            metadata_result.total_items,
            page as i64,
            per_page as i64,
        );

        // 7. Set state back to idle
        let _ = self.metadata_service.complete_sync(key);

        // Get total items from metadata service
        let total_items = self
            .metadata_service
            .get_entity_ids(key)
            .map(|ids| ids.len())
            .unwrap_or(0);

        let has_more_pages = self.metadata_service.has_more_pages(key).unwrap_or(false);

        Ok(SyncResult::new(
            total_items,
            fetched_count,
            failed_count,
            has_more_pages,
            page,
            metadata_result.total_pages,
        ))
    }

    /// Fetch full post data for specific post IDs and save to cache.
    ///
    /// This is used for selective sync - fetching only the posts that are
    /// missing or stale in the cache. Uses the `include` parameter to batch
    /// multiple posts in a single request.
    ///
    /// # State Tracking
    ///
    /// This method updates the entity state store:
    /// 1. Filters out IDs that are already `Fetching` (prevents duplicate requests)
    /// 2. Sets remaining IDs to `Fetching` before the API call
    /// 3. On success: Sets fetched posts to `Cached`, missing posts to `Failed`
    /// 4. On error: Sets all requested posts to `Failed`
    ///
    /// # Arguments
    /// * `ids` - Post IDs to fetch
    ///
    /// # Returns
    /// - `Ok(Vec<EntityId>)` with entity IDs of fetched posts
    /// - `Err(FetchError)` if network or database error occurs
    ///
    /// # Note
    /// If `ids` is empty or all IDs are already fetching, returns an empty Vec
    /// without making a network request.
    pub async fn fetch_posts_by_ids(&self, ids: Vec<PostId>) -> Result<Vec<EntityId>, FetchError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        // Convert to raw IDs and filter out already-fetching
        let raw_ids: Vec<i64> = ids.iter().map(|id| id.0).collect();
        let fetchable = self.state_store_with_edit_context.filter_fetchable(&raw_ids);

        if fetchable.is_empty() {
            return Ok(Vec::new());
        }

        // Mark as fetching
        self.state_store_with_edit_context.set_batch(&fetchable, EntityState::Fetching);

        // Convert back to PostId for the API call
        let post_ids: Vec<PostId> = fetchable.iter().map(|&id| PostId(id)).collect();

        let params = PostListParams {
            include: post_ids,
            // Ensure we get all requested posts regardless of default per_page
            per_page: Some(100),
            // Include all statuses - WordPress defaults to 'publish' which would
            // filter out drafts, pending, etc. when fetching by ID
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
            .list_with_edit_context(&PostEndpointType::Posts, &params)
            .await
        {
            Ok(response) => {
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
                        .collect::<Result<Vec<_>, _>>()
                })?;

                // Mark successfully fetched posts as Cached
                let fetched_ids: Vec<i64> = response.data.iter().map(|p| p.id.0).collect();
                self.state_store_with_edit_context
                    .set_batch(&fetched_ids, EntityState::Cached);

                // Mark posts that were requested but not returned as Failed
                let failed_ids: Vec<i64> = fetchable
                    .iter()
                    .filter(|id| !fetched_ids.contains(id))
                    .copied()
                    .collect();
                if !failed_ids.is_empty() {
                    self.state_store_with_edit_context
                        .set_batch(&failed_ids, EntityState::failed("Not found"));
                }

                Ok(entity_ids)
            }
            Err(e) => {
                // Mark all as failed
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

    /// Get direct access to the metadata service.
    ///
    /// Used when you need both read and write access to list metadata.
    pub fn metadata_service(&self) -> Arc<MetadataService> {
        self.metadata_service.clone()
    }

    /// Get the current state for a post (edit context).
    ///
    /// Returns `EntityState::Missing` if no state has been recorded.
    pub fn get_entity_state_with_edit_context(&self, post_id: PostId) -> EntityState {
        self.state_store_with_edit_context.get(post_id.0)
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
                .map(|&id| PostId(id))
                .map(|post_id| repo.select_by_post_id(connection, &self.db_site, post_id))
                .collect::<Result<Vec<_>, _>>()
                .map(|options| {
                    options
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
    /// * `filter` - Filter criteria for posts (status, etc.)
    ///
    /// # Example (Kotlin)
    /// ```kotlin
    /// val filter = AnyPostFilter(status = PostStatus.DRAFT)
    /// val collection = postService.createPostMetadataCollectionWithEditContext(filter)
    ///
    /// // Initial load - fetches metadata, then syncs missing items
    /// collection.refresh()
    ///
    /// // Get items with states and data
    /// val items = collection.loadItems()
    /// ```
    pub fn create_post_metadata_collection_with_edit_context(
        self: &Arc<Self>,
        filter: AnyPostFilter,
    ) -> PostMetadataCollectionWithEditContext {
        // TODO: Implement proper cache key generation based on filter
        // For now, use a simple key based on status
        let kv_key = format!(
            "site_{:?}:edit:posts:{}",
            self.db_site.row_id,
            filter
                .status
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "all".to_string())
        );

        let fetcher = PersistentPostMetadataFetcherWithEditContext::new(
            self.clone(),
            filter.clone(),
            kv_key.clone(),
        );

        let metadata_collection = MetadataCollection::new(
            kv_key,
            self.persistent_metadata_reader(),
            self.state_reader_with_edit_context(),
            fetcher,
            vec![
                DbTable::PostsEditContext,
                DbTable::TermRelationships,
                DbTable::ListMetadataItems,
            ],
        );

        PostMetadataCollectionWithEditContext::new(metadata_collection, self.clone(), filter)
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
