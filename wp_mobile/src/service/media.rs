use crate::{
    EntityMediaWithEditContext,
    cache_key::media_list_filter_cache_key,
    collection::{
        FetchError, FetchResult, MediaMetadataCollectionWithEditContext, MetadataCollectionCore,
    },
    filters::MediaListFilter,
    service::{
        entity_state_service::{EntityStateReader, EntityStateReaderImpl, EntityStateService},
        metadata::MetadataService,
    },
    sync::{DbEntityState, EntityMetadata, MetadataFetchResult, SyncResult, SyncStrategy},
};
use std::{collections::HashSet, sync::Arc};
use wp_api::{
    api_client::WpApiClient,
    media::{
        MediaDeleteResponse, MediaId, MediaListParams, MediaStatus, MediaWithEditContext,
        SparseMediaFieldWithEditContext,
    },
};
use wp_mobile_cache::{
    DbTable, WpApiCache,
    context::EditContext,
    db_types::db_site::DbSite,
    entity::{Entity, EntityId, FullEntity},
    list_metadata::ListKey,
    repository::{entity_state::EntityType, media::MediaRepository},
};

/// Maximum number of media items to fetch in a single batch request
const BATCH_FETCH_SIZE: usize = 100;

/// All core WordPress attachment statuses. Used by `load_media_by_ids` to
/// bypass the REST default of `status=inherit` so we can hydrate items the
/// metadata pass returned via a status filter on the user's `MediaListFilter`.
const ALL_ATTACHMENT_STATUSES: &[MediaStatus] = &[
    MediaStatus::Inherit,
    MediaStatus::Private,
    MediaStatus::Trash,
];

// Internal types

/// Result from loading media by IDs.
pub struct LoadByIdsResult {
    /// Entity IDs of successfully loaded media
    pub entity_ids: Vec<EntityId>,
    /// Number of media items that were requested but failed to load
    pub failed_count: usize,
}

/// Statistics from fetching missing and stale media.
pub(crate) struct FetchStats {
    /// Number of media items that needed fetching (Missing or Stale state)
    pub(crate) fetched_count: usize,
    /// Number of media items that failed to fetch
    pub(crate) failed_count: usize,
}

/// Service layer for media operations
///
/// Provides a bridge between clients and the underlying network/cache layers.
/// Handles fetching and deleting media. Mutations that iOS performs directly
/// against the API client (create/update/upload) are deliberately not exposed here.
///
/// # Metadata Sync Infrastructure
///
/// The service provides access to metadata-first sync infrastructure:
/// - Entity state tracking via `EntityStateStore` associated functions
/// - Database-backed list metadata via `metadata_service`
///
/// Collections get read-only access via reader methods. This ensures cross-collection
/// consistency when multiple collections share the same underlying entities.
#[derive(uniffi::Object)]
pub struct MediaService {
    db_site: Arc<DbSite>,
    api_client: Arc<WpApiClient>,
    cache: Arc<WpApiCache>,

    /// Database-backed list metadata service.
    /// Persists list structure across app restarts.
    pub(crate) metadata_service: Arc<MetadataService>,
}

impl MediaService {
    pub fn new(api_client: Arc<WpApiClient>, db_site: Arc<DbSite>, cache: Arc<WpApiCache>) -> Self {
        let metadata_service = Arc::new(MetadataService::new(db_site.clone(), cache.clone()));

        Self {
            api_client,
            db_site,
            cache,
            metadata_service,
        }
    }

    /// Sync a page of media items from network to cache.
    ///
    /// Fetches full media data from the API and saves it to the database:
    /// 1. Converts filter to API parameters
    /// 2. Makes network request via WpApiClient
    /// 3. Upserts media to database via repository
    /// 4. Returns entity IDs and pagination info
    pub async fn sync_media_page(
        &self,
        filter: &MediaListFilter,
        page: u32,
        per_page: u32,
    ) -> Result<FetchResult, FetchError> {
        let params = filter.to_list_params(page, per_page);

        let response = self
            .api_client
            .media()
            .list_with_edit_context(&params)
            .await?;

        let entity_ids = self.cache.execute(|conn| {
            let repo = MediaRepository::<EditContext>::new();
            response
                .data
                .iter()
                .map(|media| {
                    repo.upsert(conn, &self.db_site, media)
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

    /// Fetch lightweight metadata (id, modified_gmt, post_id) for a page of media.
    ///
    /// Returns only the minimal fields needed to determine list structure and staleness.
    /// Does not fetch or save full media content.
    pub(crate) async fn fetch_media_metadata(
        &self,
        filter: &MediaListFilter,
        page: u32,
        per_page: u32,
    ) -> Result<MetadataFetchResult, FetchError> {
        let request_params = filter.to_list_params(page, per_page);

        let response = self
            .api_client
            .media()
            .filter_list_with_edit_context(
                &request_params,
                &[
                    SparseMediaFieldWithEditContext::Id,
                    SparseMediaFieldWithEditContext::ModifiedGmt,
                    SparseMediaFieldWithEditContext::PostId,
                ],
            )
            .await?;

        // `post_id` (media's "attached post") serves the parent slot in `EntityMetadata`.
        // `menu_order` is always `None` for media.
        let metadata: Vec<EntityMetadata> = response
            .data
            .into_iter()
            .filter_map(|sparse| {
                Some(EntityMetadata::new(
                    sparse.id?.0,
                    sparse.modified_gmt,
                    sparse.post_id.map(|p| p.0),
                    None,
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

    /// Find stale media items by comparing fetched metadata timestamps with cached DB values.
    ///
    /// A media item is considered stale if:
    /// 1. It's currently in `Fresh` state in the state store
    /// 2. Its fetched `modified_gmt` differs from the cached `modified_gmt` in the database
    pub(crate) fn find_stale_media_by_timestamp(
        &self,
        metadata: &[EntityMetadata],
        state_reader: &dyn EntityStateReader,
    ) -> Vec<i64> {
        let cached_ids: Vec<MediaId> = metadata
            .iter()
            .filter(|m| matches!(state_reader.get(m.id), DbEntityState::Fresh))
            .map(|m| MediaId(m.id))
            .collect();

        if cached_ids.is_empty() {
            return Vec::new();
        }

        let cached_timestamps = self
            .cache
            .execute(|conn| {
                let repo = MediaRepository::<EditContext>::new();
                repo.select_modified_gmt_by_ids(conn, &self.db_site, &cached_ids)
            })
            .unwrap_or_else(|e| {
                log::warn!(
                    "Failed to query cached timestamps for staleness check: {}",
                    e
                );
                Default::default()
            });

        metadata
            .iter()
            .filter_map(|m| {
                if let Some(fetched_modified) = &m.modified_gmt
                    && let Some(cached_modified) = cached_timestamps.get(&MediaId(m.id))
                    && fetched_modified != cached_modified
                {
                    Some(m.id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Sync a media list using the default full sync strategy.
    pub async fn sync_list(
        &self,
        key: &ListKey,
        filter: &MediaListFilter,
        per_page: u32,
        is_refresh: bool,
    ) -> Result<SyncResult, FetchError> {
        self.sync_list_with_strategy(key, filter, per_page, is_refresh, SyncStrategy::Full)
            .await
    }

    /// Sync a media list with explicit strategy control.
    ///
    /// Mirrors `PostService::sync_list_with_strategy`: fetches list metadata, stores
    /// it, detects stale items via modified_gmt comparison, and (for `Full`) selectively
    /// fetches missing/stale entities.
    pub async fn sync_list_with_strategy(
        &self,
        key: &ListKey,
        filter: &MediaListFilter,
        per_page: u32,
        is_refresh: bool,
        strategy: SyncStrategy,
    ) -> Result<SyncResult, FetchError> {
        // 1. Fetch and store metadata
        let metadata_result = if is_refresh {
            self.metadata_service
                .refresh(key, per_page, |page, per_page| {
                    self.fetch_media_metadata(filter, page, per_page)
                })
                .await?
        } else {
            self.metadata_service
                .load_more(key, |page, per_page| {
                    self.fetch_media_metadata(filter, page, per_page)
                })
                .await?
        };

        // 2. Detect and mark stale media (always done - doesn't fetch)
        let stale_ids = self.find_stale_media_by_timestamp(
            &metadata_result.metadata,
            self.state_reader_with_edit_context().as_ref(),
        );

        if !stale_ids.is_empty() {
            log::debug!(
                "Found {} stale media item(s) via modified_gmt comparison",
                stale_ids.len()
            );
            EntityStateService::save_batch(
                &self.cache,
                &self.db_site,
                EntityType::MediaEditContext,
                &stale_ids,
                DbEntityState::Stale,
            );
        }

        // 3. Fetch missing/stale media (only for Full strategy)
        let stats = match strategy {
            SyncStrategy::MetadataOnly => FetchStats {
                fetched_count: 0,
                failed_count: 0,
            },
            SyncStrategy::Full => {
                self.fetch_missing_and_stale_media(&metadata_result.metadata)
                    .await
            }
        };

        let total_items = self
            .metadata_service
            .get_entity_ids(key)
            .map(|ids| ids.len())
            .unwrap_or(0);

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

    /// Fetch media items that are missing or stale based on current state.
    pub(crate) async fn fetch_missing_and_stale_media(
        &self,
        metadata: &[EntityMetadata],
    ) -> FetchStats {
        let ids_to_fetch: Vec<MediaId> = metadata
            .iter()
            .filter(|m| {
                let state = EntityStateService::get(
                    &self.cache,
                    &self.db_site,
                    EntityType::MediaEditContext,
                    m.id,
                );
                state.needs_fetch()
            })
            .map(|m| MediaId(m.id))
            .collect();

        let fetched_count = ids_to_fetch.len();
        let mut failed_count = 0;

        if !ids_to_fetch.is_empty() {
            for chunk in ids_to_fetch.chunks(BATCH_FETCH_SIZE) {
                match self.load_media_by_ids(chunk.to_vec()).await {
                    Ok(result) => {
                        failed_count += result.failed_count;
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to load {} media item(s) (IDs: {:?}): {}",
                            chunk.len(),
                            chunk,
                            e
                        );
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

    /// Load media items by IDs from network to cache with state tracking.
    ///
    /// Mirrors `PostService::load_posts_by_ids`. Passes `ALL_ATTACHMENT_STATUSES`
    /// explicitly so the REST controller's `status=inherit` default doesn't filter
    /// out IDs the metadata pass surfaced via a `status` filter (e.g. `Private` or
    /// `Trash`).
    pub async fn load_media_by_ids(
        &self,
        ids: Vec<MediaId>,
    ) -> Result<LoadByIdsResult, FetchError> {
        if ids.is_empty() {
            return Ok(LoadByIdsResult {
                entity_ids: Vec::new(),
                failed_count: 0,
            });
        }

        let raw_ids: Vec<i64> = ids.iter().map(|id| id.0).collect();
        let fetchable = EntityStateService::filter_fetchable(
            &self.cache,
            &self.db_site,
            EntityType::MediaEditContext,
            &raw_ids,
        );

        if fetchable.is_empty() {
            return Ok(LoadByIdsResult {
                entity_ids: Vec::new(),
                failed_count: 0,
            });
        }

        EntityStateService::save_batch(
            &self.cache,
            &self.db_site,
            EntityType::MediaEditContext,
            &fetchable,
            DbEntityState::Fetching,
        );

        let media_ids: Vec<MediaId> = fetchable.iter().map(|&id| MediaId(id)).collect();

        let params = MediaListParams {
            include: media_ids,
            per_page: Some(BATCH_FETCH_SIZE as u32),
            status: ALL_ATTACHMENT_STATUSES.to_vec(),
            ..Default::default()
        };

        match self
            .api_client
            .media()
            .list_with_edit_context(&params)
            .await
        {
            Ok(response) => {
                let entity_ids = match self.cache.execute(|conn| {
                    let repo = MediaRepository::<EditContext>::new();
                    response
                        .data
                        .iter()
                        .map(|media| {
                            repo.upsert(conn, &self.db_site, media).map_err(|e| {
                                FetchError::Database {
                                    err_message: e.to_string(),
                                }
                            })
                        })
                        .collect::<Result<Vec<EntityId>, FetchError>>()
                }) {
                    Ok(ids) => ids,
                    Err(e) => {
                        EntityStateService::save_batch(
                            &self.cache,
                            &self.db_site,
                            EntityType::MediaEditContext,
                            &fetchable,
                            DbEntityState::failed(e.to_string()),
                        );
                        return Err(e);
                    }
                };

                let fetched_ids: Vec<i64> = response.data.iter().map(|m| m.id.0).collect();
                EntityStateService::save_batch(
                    &self.cache,
                    &self.db_site,
                    EntityType::MediaEditContext,
                    &fetched_ids,
                    DbEntityState::Fresh,
                );

                let fetched_set: HashSet<i64> = fetched_ids.iter().copied().collect();
                let failed_ids: Vec<i64> = fetchable
                    .iter()
                    .filter(|id| !fetched_set.contains(id))
                    .copied()
                    .collect();
                let failed_count = failed_ids.len();
                if !failed_ids.is_empty() {
                    EntityStateService::save_batch(
                        &self.cache,
                        &self.db_site,
                        EntityType::MediaEditContext,
                        &failed_ids,
                        DbEntityState::failed("Not found"),
                    );
                }

                Ok(LoadByIdsResult {
                    entity_ids,
                    failed_count,
                })
            }
            Err(e) => {
                EntityStateService::save_batch(
                    &self.cache,
                    &self.db_site,
                    EntityType::MediaEditContext,
                    &fetchable,
                    DbEntityState::failed(e.to_string()),
                );
                Err(e.into())
            }
        }
    }

    /// Get read-only access to the entity state reader for edit context.
    pub fn state_reader_with_edit_context(&self) -> Arc<dyn EntityStateReader> {
        Arc::new(EntityStateReaderImpl::new(
            self.cache.clone(),
            *self.db_site,
            EntityType::MediaEditContext,
        ))
    }

    /// Get read-only access to the persistent metadata service.
    pub fn persistent_metadata_reader(&self) -> Arc<MetadataService> {
        self.metadata_service.clone()
    }

    /// Read media items by IDs from the database cache.
    pub fn read_media_by_ids_from_db(
        &self,
        ids: &[i64],
    ) -> Result<Vec<FullEntity<MediaWithEditContext>>, wp_mobile_cache::SqliteDbError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let repo = MediaRepository::<EditContext>::new();

        // TODO: query database for all IDs in one call instead of iterating?
        self.cache.execute(|connection| {
            ids.iter()
                .map(|&id| repo.select_by_media_id(connection, &self.db_site, MediaId(id)))
                .collect::<Result<Vec<_>, _>>()
                .map(|items| {
                    items
                        .into_iter()
                        .flatten()
                        .map(|db_media| FullEntity::new(db_media.entity_id, db_media.data.media))
                        .collect()
                })
        })
    }
}

#[uniffi::export]
impl MediaService {
    /// Get an entity handle using an EntityId
    ///
    /// Returns an entity that can be used to read media data with full edit context.
    /// The entity is lightweight - it doesn't fetch data until you call load_data() on it.
    pub fn get_entity_with_edit_context(&self, entity_id: EntityId) -> EntityMediaWithEditContext {
        let cache = self.cache.clone();

        Entity::<MediaWithEditContext>::new(
            entity_id,
            Box::new(move || {
                let repo = MediaRepository::<EditContext>::new();

                cache
                    .execute(|connection| repo.select_by_entity_id(connection, &entity_id))
                    .map(|opt| {
                        opt.map(|db_media_full_entity| {
                            FullEntity::new(
                                db_media_full_entity.entity_id,
                                db_media_full_entity.data.media,
                            )
                        })
                    })
            }),
        )
        .into()
    }

    /// Get the total count of media for this site
    pub fn count_edit_context(&self) -> Result<i64, wp_mobile_cache::SqliteDbError> {
        let repo = MediaRepository::<EditContext>::new();
        self.cache
            .execute(|connection| repo.count(connection, &self.db_site))
    }

    /// Delete a media item by its EntityId
    pub fn delete_by_entity_id(
        &self,
        entity_id: &EntityId,
    ) -> Result<u64, wp_mobile_cache::SqliteDbError> {
        let repo = MediaRepository::<EditContext>::new();
        self.cache.execute(|connection| {
            repo.delete_by_entity_id(connection, entity_id)
                .map(|n| n as u64)
        })
    }

    /// Delete a media item by its WordPress media ID
    pub fn delete_by_media_id(
        &self,
        media_id: MediaId,
    ) -> Result<u64, wp_mobile_cache::SqliteDbError> {
        let repo = MediaRepository::<EditContext>::new();
        self.cache.execute(|connection| {
            repo.delete_by_media_id(connection, &self.db_site, media_id)
                .map(|n| n as u64)
        })
    }

    /// Permanently delete a media item via the REST API and remove it from the local cache.
    ///
    /// REST DELETE on media always passes `force=true` (the server rejects `force=false`),
    /// so there is no separate "trash" path.
    pub async fn delete_media_permanently(
        self: &Arc<Self>,
        media_id: &MediaId,
    ) -> Result<MediaDeleteResponse, FetchError> {
        let response = self.api_client.media().delete(media_id).await?.data;

        self.delete_by_media_id(*media_id)
            .map_err(|e| FetchError::Database {
                err_message: e.to_string(),
            })?;

        // Scrub the deleted media from every cached media list. Without this,
        // collection load_items would return a phantom row that converts to
        // `Missing`/`Stale` until the next full refresh. Failure here is
        // logged but not propagated: the REST delete and local row delete
        // already succeeded.
        let media_list_prefix = format!("site_{:?}:edit:media:", self.db_site.row_id);
        if let Err(e) = self
            .metadata_service
            .remove_entity_from_lists_with_key_prefix(&media_list_prefix, media_id.0)
        {
            log::warn!(
                "Failed to remove deleted media id {} from list metadata: {}",
                media_id.0,
                e
            );
        }

        Ok(response)
    }

    /// Create a metadata-first media collection with edit context
    ///
    /// Returns a collection that uses a two-phase sync strategy:
    /// 1. Fetch lightweight metadata (id + modified_gmt) to define list structure
    /// 2. Selectively fetch full data for missing or stale items
    pub fn create_media_metadata_collection_with_edit_context(
        self: &Arc<Self>,
        filter: MediaListFilter,
        per_page: u32,
    ) -> Arc<MediaMetadataCollectionWithEditContext> {
        let cache_key = media_list_filter_cache_key(&filter);
        let key: ListKey =
            format!("site_{:?}:edit:media:{}", self.db_site.row_id, cache_key).into();

        let core = MetadataCollectionCore::new(
            key,
            self.persistent_metadata_reader(),
            self.state_reader_with_edit_context(),
            vec![DbTable::MediaEditContext, DbTable::ListMetadataItems],
            per_page,
        );

        Arc::new(MediaMetadataCollectionWithEditContext::new(
            core,
            self.clone(),
            filter,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{EmptyAppNotifier, MockExecutor, mock_api_client};
    use rstest::*;
    use rusqlite::Connection;
    use wp_api::{media::MediaId, prelude::*};
    use wp_mobile_cache::{
        MigrationManager, WpApiCache,
        db_types::self_hosted_site::SelfHostedSite,
        repository::{media::MediaRepository, sites::SiteRepository},
        test_fixtures::media::MediaBuilder,
    };

    /// Test context bundling MediaService with database and site setup
    pub struct MediaServiceTestContext {
        pub media_service: Arc<MediaService>,
        pub db_site: Arc<DbSite>,
        pub cache: Arc<WpApiCache>,
    }

    /// Test helper that encapsulates a test media item with its assertion logic
    struct TestMedia {
        id: MediaId,
        title: String,
        slug: String,
    }

    impl TestMedia {
        fn assert_matches(&self, media: &MediaWithEditContext) {
            assert_eq!(media.id, self.id);
            assert_eq!(media.title.rendered, self.title);
            assert_eq!(media.slug, self.slug);
        }
    }

    /// Helper function to insert a test media item into the cache.
    fn insert_test_media(ctx: &MediaServiceTestContext) -> TestMedia {
        let test_media = TestMedia {
            id: MediaId(4242),
            title: "Test Media".to_string(),
            slug: "test-media".to_string(),
        };

        let media = MediaBuilder::minimal()
            .with_id(test_media.id.0)
            .with_title(&test_media.title)
            .with_slug(&test_media.slug)
            .build();

        ctx.cache
            .execute(|conn| {
                let repo = MediaRepository::<EditContext>::new();
                repo.upsert(conn, &ctx.db_site, &media)
            })
            .expect("Media insert should succeed");

        test_media
    }

    #[rstest]
    fn test_get_entity_load_data_returns_cached_media(ctx: MediaServiceTestContext) {
        let test_media = insert_test_media(&ctx);

        let entity_id = ctx
            .cache
            .execute(|conn| {
                let repo = MediaRepository::<EditContext>::new();
                repo.select_by_media_id(conn, &ctx.db_site, test_media.id)
                    .map(|opt| opt.map(|full_entity| *full_entity.entity_id))
            })
            .expect("Database read should succeed")
            .expect("Media should exist");

        let entity = ctx.media_service.get_entity_with_edit_context(entity_id);
        let result = entity.0.load_data().expect("Database read should succeed");

        let full_entity = result.expect("Media should be found in cache");
        test_media.assert_matches(&full_entity.data);
    }

    #[rstest]
    fn test_delete_by_entity_id(ctx: MediaServiceTestContext) {
        let test_media = insert_test_media(&ctx);
        let entity_id = ctx
            .cache
            .execute(|conn| {
                let repo = MediaRepository::<EditContext>::new();
                repo.select_by_media_id(conn, &ctx.db_site, test_media.id)
                    .map(|opt| opt.map(|full_entity| *full_entity.entity_id))
            })
            .expect("Database read should succeed")
            .expect("Media should exist");

        let deleted = ctx
            .media_service
            .delete_by_entity_id(&entity_id)
            .expect("Delete should succeed");

        assert_eq!(deleted, 1, "Should delete 1 media item");

        let result = ctx.cache.execute(|conn| {
            let repo = MediaRepository::<EditContext>::new();
            repo.select_by_entity_id(conn, &entity_id)
        });
        assert!(
            result.unwrap().is_none(),
            "Media should not exist after deletion"
        );
    }

    #[rstest]
    fn test_delete_by_media_id(ctx: MediaServiceTestContext) {
        let test_media = insert_test_media(&ctx);

        let deleted = ctx
            .media_service
            .delete_by_media_id(test_media.id)
            .expect("Delete should succeed");

        assert_eq!(deleted, 1, "Should delete 1 media item");

        let result = ctx.cache.execute(|conn| {
            let repo = MediaRepository::<EditContext>::new();
            repo.select_by_media_id(conn, &ctx.db_site, test_media.id)
        });
        assert!(
            result.unwrap().is_none(),
            "Media should not exist after deletion"
        );
    }

    #[rstest]
    fn test_delete_by_entity_id_non_existent_returns_zero(ctx: MediaServiceTestContext) {
        let test_media = insert_test_media(&ctx);
        let entity_id = ctx
            .cache
            .execute(|conn| {
                let repo = MediaRepository::<EditContext>::new();
                repo.select_by_media_id(conn, &ctx.db_site, test_media.id)
                    .map(|opt| opt.map(|full_entity| *full_entity.entity_id))
            })
            .expect("Database read should succeed")
            .expect("Media should exist");

        ctx.media_service
            .delete_by_entity_id(&entity_id)
            .expect("First delete should succeed");

        let deleted = ctx
            .media_service
            .delete_by_entity_id(&entity_id)
            .expect("Delete should not error");

        assert_eq!(deleted, 0, "Should return 0 for non-existent media");
    }

    #[rstest]
    fn test_delete_by_media_id_non_existent_returns_zero(ctx: MediaServiceTestContext) {
        let deleted = ctx
            .media_service
            .delete_by_media_id(MediaId(99999))
            .expect("Delete should not error");

        assert_eq!(deleted, 0, "Should return 0 for non-existent media");
    }

    /// Helper to create a MediaService whose network requests always fail.
    fn service_with_network_error() -> Arc<MediaService> {
        let mock_executor = Arc::new(MockExecutor::with_execute_fn(|request| {
            Err(RequestExecutionError::RequestExecutionFailed {
                status_code: None,
                redirects: None,
                reason: RequestExecutionErrorReason::GenericError {
                    error_message: "Network timeout".to_string(),
                },
                request_url: request.url().0,
                request_method: request.method(),
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

        let cache = Arc::new(WpApiCache::try_from(conn).expect("Cache creation should succeed"));
        let db_site_arc = Arc::new(db_site);
        Arc::new(MediaService::new(api_client, db_site_arc, cache))
    }

    #[tokio::test]
    async fn test_load_media_by_ids_includes_all_attachment_statuses_in_request() {
        // The metadata pass can use `MediaListFilter.status = [Private]`, so the
        // hydration follow-up via `include` must explicitly pass every core
        // attachment status to bypass the REST controller's `status=inherit`
        // default. Otherwise the included IDs get filtered back out and end up
        // marked Failed("Not found").
        let service = service_with_network_error();

        let result = service
            .load_media_by_ids(vec![MediaId(1), MediaId(2)])
            .await;

        let request_url = match result {
            Err(FetchError::Api(WpApiError::RequestExecutionFailed { request_url, .. })) => {
                request_url
            }
            Err(other) => panic!("Expected RequestExecutionFailed, got: {:?}", other),
            Ok(_) => panic!("Expected network error, got Ok"),
        };

        // URL-encoded comma is %2C
        assert!(
            request_url.contains("status=inherit%2Cprivate%2Ctrash"),
            "expected request URL to include status=inherit,private,trash; got {}",
            request_url
        );
    }

    #[tokio::test]
    async fn test_load_media_by_ids_marks_all_as_failed_on_network_error() {
        let service = service_with_network_error();

        let result = service
            .load_media_by_ids(vec![MediaId(1), MediaId(2)])
            .await;

        assert!(result.is_err(), "Network error should return Err");

        let state1 = EntityStateService::get(
            &service.cache,
            &service.db_site,
            EntityType::MediaEditContext,
            1,
        );
        let state2 = EntityStateService::get(
            &service.cache,
            &service.db_site,
            EntityType::MediaEditContext,
            2,
        );
        assert!(
            matches!(state1, crate::sync::DbEntityState::Failed { .. }),
            "Media 1 should be marked as Failed on network error"
        );
        assert!(
            matches!(state2, crate::sync::DbEntityState::Failed { .. }),
            "Media 2 should be marked as Failed on network error"
        );
    }

    #[rstest]
    fn test_create_media_metadata_collection_with_edit_context_returns_arc(
        ctx: MediaServiceTestContext,
    ) {
        // Sanity check: the factory wires the collection without panicking.
        let _collection = ctx
            .media_service
            .create_media_metadata_collection_with_edit_context(MediaListFilter::default(), 20);
    }

    /// Tests the cleanup helper directly (approach b in the bug-fix spec).
    ///
    /// Going through `delete_media_permanently` would require mocking a valid
    /// `MediaDeleteResponse` JSON which adds a lot of brittle setup, so this
    /// test covers the cleanup helper that `delete_media_permanently` calls.
    #[rstest]
    fn test_remove_entity_from_lists_with_key_prefix_only_removes_from_matching_keys(
        ctx: MediaServiceTestContext,
    ) {
        use wp_mobile_cache::repository::list_metadata::{
            ListMetadataItemInput, ListMetadataRepository,
        };

        let media_key: ListKey =
            format!("site_{:?}:edit:media:filter=fake", ctx.db_site.row_id).into();
        let posts_key: ListKey = format!("site_{:?}:edit:posts:foo", ctx.db_site.row_id).into();
        let entity_id: i64 = 42;

        // Seed two list_metadata rows (one media-prefixed, one posts-prefixed)
        // and put `entity_id` into both.
        ctx.cache
            .execute(|conn| {
                let item = ListMetadataItemInput {
                    entity_id,
                    modified_gmt: None,
                    parent: None,
                    menu_order: None,
                };
                ListMetadataRepository::set_items_by_list_key(
                    conn,
                    &ctx.db_site,
                    &media_key,
                    25,
                    std::slice::from_ref(&item),
                )?;
                ListMetadataRepository::set_items_by_list_key(
                    conn,
                    &ctx.db_site,
                    &posts_key,
                    25,
                    std::slice::from_ref(&item),
                )
            })
            .expect("Seeding list metadata should succeed");

        // Sanity: both lists reference the entity before cleanup.
        assert!(
            ctx.media_service
                .metadata_service
                .list_contains_entity(&media_key, entity_id)
                .expect("contains check"),
            "media list should contain the entity before cleanup"
        );
        assert!(
            ctx.media_service
                .metadata_service
                .list_contains_entity(&posts_key, entity_id)
                .expect("contains check"),
            "posts list should contain the entity before cleanup"
        );

        // Act: scrub the entity from media-prefixed lists only.
        let media_list_prefix = format!("site_{:?}:edit:media:", ctx.db_site.row_id);
        let removed = ctx
            .media_service
            .metadata_service
            .remove_entity_from_lists_with_key_prefix(&media_list_prefix, entity_id)
            .expect("cleanup should succeed");
        assert_eq!(removed, 1, "should remove exactly one row");

        // Media list no longer references 42.
        assert!(
            !ctx.media_service
                .metadata_service
                .list_contains_entity(&media_key, entity_id)
                .expect("contains check"),
            "media list should NOT contain the entity after cleanup"
        );
        // Posts list still references 42 (no over-match on the prefix).
        assert!(
            ctx.media_service
                .metadata_service
                .list_contains_entity(&posts_key, entity_id)
                .expect("contains check"),
            "posts list should still contain the entity (prefix-scoped delete must not over-match)"
        );
    }

    #[fixture]
    fn ctx(mock_api_client: Arc<WpApiClient>) -> MediaServiceTestContext {
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
        let db_site_arc = Arc::new(db_site);
        let media_service = Arc::new(MediaService::new(
            mock_api_client,
            db_site_arc.clone(),
            cache.clone(),
        ));

        MediaServiceTestContext {
            media_service,
            db_site: db_site_arc,
            cache,
        }
    }
}
