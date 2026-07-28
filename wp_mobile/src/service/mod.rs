use crate::service::sites::SiteInfo;
use crate::service::{
    media::MediaService, post_types::PostTypeService, posts::PostService, sites::SiteService,
};
use std::sync::Arc;
use wp_api::prelude::{ApiUrlResolver, WpApiClient, WpApiClientDelegate};
use wp_api::request::RequestExecutor;
use wp_mobile_cache::{WpApiCache, db_types::db_site::DbSite};

pub mod entity_state_service;
pub mod media;
pub mod metadata;
pub mod mock_post_service;
pub mod post_types;
pub mod posts;
pub mod sites;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum WpServiceError {
    #[error("Database error: {err_message}")]
    DatabaseError { err_message: String },

    #[error("Invalid URL: {err_message}")]
    InvalidUrl { err_message: String },

    #[error("Site not found in cache")]
    SiteNotFound,
}

impl From<wp_mobile_cache::SqliteDbError> for WpServiceError {
    fn from(err: wp_mobile_cache::SqliteDbError) -> Self {
        WpServiceError::DatabaseError {
            err_message: err.to_string(),
        }
    }
}

/// Service for a WordPress site
///
/// This service coordinates between the API client and cache for a specific
/// WordPress site (self-hosted or WordPress.com). It provides access to
/// domain-specific services like PostService, PostTypeService, etc.
#[derive(uniffi::Object)]
pub struct WpService {
    media: Arc<MediaService>,
    post_types: Arc<PostTypeService>,
    posts: Arc<PostService>,
    sites: Arc<SiteService>,
    request_executor: Arc<dyn RequestExecutor>,
}

impl WpService {
    fn build_services(
        api_url_resolver: Arc<dyn ApiUrlResolver>,
        delegate: WpApiClientDelegate,
        cache: Arc<WpApiCache>,
        db_site: DbSite,
        site_service: Arc<SiteService>,
    ) -> Self {
        let request_executor = delegate.request_executor.clone();
        let api_client = Arc::new(WpApiClient::new(api_url_resolver, delegate));
        let db_site_arc = Arc::new(db_site);

        let media = Arc::new(MediaService::new(
            api_client.clone(),
            db_site_arc.clone(),
            cache.clone(),
        ));
        let posts = Arc::new(PostService::new(
            api_client.clone(),
            db_site_arc.clone(),
            cache.clone(),
        ));
        let post_types = Arc::new(PostTypeService::new(api_client, db_site_arc, cache));

        Self {
            media,
            post_types,
            posts,
            sites: site_service,
            request_executor,
        }
    }
}

#[uniffi::export]
impl WpService {
    /// Create a new service for a WordPress site.
    #[uniffi::constructor]
    pub fn new(
        site_info: SiteInfo,
        delegate: WpApiClientDelegate,
        cache: Arc<WpApiCache>,
    ) -> Result<Self, WpServiceError> {
        let api_url_resolver = site_info.api_url_resolver();
        let db_site = match &site_info {
            SiteInfo::SelfHosted { site_url, api_root } => {
                SiteService::get_or_create_self_hosted_site(
                    cache.clone(),
                    site_url.url(),
                    api_root.url(),
                )?
            }
            SiteInfo::WordPressCom { site_id } => {
                SiteService::get_or_create_wordpress_com_site(cache.clone(), *site_id)?
            }
        };
        let sites = Arc::new(SiteService::new(cache.clone(), db_site));
        Ok(Self::build_services(
            api_url_resolver,
            delegate,
            cache,
            db_site,
            sites,
        ))
    }

    /// Get the media service for this WordPress site
    pub fn media(&self) -> Arc<MediaService> {
        self.media.clone()
    }

    /// Get the post type service for this WordPress site
    pub fn post_types(&self) -> Arc<PostTypeService> {
        self.post_types.clone()
    }

    /// Get the post service for this WordPress site
    pub fn posts(&self) -> Arc<PostService> {
        self.posts.clone()
    }

    /// Get the site service for this WordPress site
    pub fn sites(&self) -> Arc<SiteService> {
        self.sites.clone()
    }

    /// Remove this site's cached data.
    ///
    /// Returns `true` if the site existed in the cache and `false` if it had
    /// already been removed. After successful removal, this service should be
    /// discarded rather than reused.
    pub fn remove_cached_data(&self) -> Result<bool, WpServiceError> {
        self.sites.remove_cached_data()
    }

    /// Get the request executor used by this service.
    pub fn request_executor(&self) -> Arc<dyn RequestExecutor> {
        self.request_executor.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{WpService, sites::SiteInfo};
    use crate::testing::{EmptyAppNotifier, MockExecutor};
    use rusqlite::params;
    use std::sync::Arc;
    use wp_api::prelude::{
        ParsedUrl, WpApiClientDelegate, WpApiMiddlewarePipeline, WpAuthenticationProvider,
    };
    use wp_api::wp_com::WpComSiteId;
    use wp_mobile_cache::{
        WpApiCache,
        db_types::db_site::DbSite,
        repository::{
            entity_state::{DbEntityState, EntityStateRepository, EntityType},
            sites::SiteRepository,
        },
    };

    fn test_delegate() -> WpApiClientDelegate {
        WpApiClientDelegate {
            auth_provider: Arc::new(WpAuthenticationProvider::none()),
            request_executor: Arc::new(MockExecutor::with_execute_fn(|_| {
                panic!("Network request should not be made while deleting cached data")
            })),
            middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
            app_notifier: Arc::new(EmptyAppNotifier),
        }
    }

    fn test_cache() -> Arc<WpApiCache> {
        let connection =
            rusqlite::Connection::open_in_memory().expect("in-memory connection should be created");
        connection
            .pragma_update(None, "foreign_keys", "OFF")
            .expect("foreign keys should be disabled before cache creation");
        wp_mobile_cache::MigrationManager::new(&connection)
            .expect("migration manager should be created")
            .perform_migrations()
            .expect("cache migrations should succeed");

        let cache =
            Arc::new(WpApiCache::try_from(connection).expect("in-memory cache should be created"));
        cache.execute(|connection| {
            connection
                .pragma_update(None, "foreign_keys", "OFF")
                .expect("foreign keys should be disabled for the test");
        });
        cache
    }

    fn foreign_key_enforcement_is_enabled(cache: &WpApiCache) -> bool {
        cache.execute(|connection| {
            connection
                .pragma_query_value(None, "foreign_keys", |row| row.get::<_, bool>(0))
                .expect("foreign key setting should be readable")
        })
    }

    fn self_hosted_service(cache: Arc<WpApiCache>, site_url: &str) -> WpService {
        WpService::new(
            SiteInfo::SelfHosted {
                site_url: Arc::new(ParsedUrl::parse(site_url).expect("site URL should parse")),
                api_root: Arc::new(
                    ParsedUrl::parse(&format!("{site_url}/wp-json"))
                        .expect("API root URL should parse"),
                ),
            },
            test_delegate(),
            cache,
        )
        .expect("self-hosted service should be created")
    }

    fn wordpress_com_service(cache: Arc<WpApiCache>, site_id: WpComSiteId) -> WpService {
        WpService::new(SiteInfo::WordPressCom { site_id }, test_delegate(), cache)
            .expect("WordPress.com service should be created")
    }

    fn seed_cached_records(cache: &WpApiCache, db_site: &DbSite) {
        cache.execute(|conn| {
            conn.execute(
                "INSERT INTO list_metadata (db_site_id, key) VALUES (?1, ?2)",
                params![db_site.row_id, "edit:posts:publish"],
            )
            .expect("list metadata should be inserted");
            EntityStateRepository::set_state(
                conn,
                42,
                db_site,
                EntityType::PostsEditContext,
                &DbEntityState::Fresh,
            )
            .expect("entity state should be inserted");
        });
    }

    fn count_rows_for_site(cache: &WpApiCache, table: &str, db_site: &DbSite) -> usize {
        cache.execute(|conn| {
            let sql = format!("SELECT COUNT(*) FROM {table} WHERE db_site_id = ?");
            conn.query_row(&sql, [db_site.row_id], |row| row.get::<_, i64>(0))
                .expect("site rows should be counted") as usize
        })
    }

    #[test]
    fn remove_cached_data_deletes_self_hosted_site_and_preserves_other_site() {
        let cache = test_cache();
        let service = self_hosted_service(cache.clone(), "https://removed.example.com");
        let _other_service = self_hosted_service(cache.clone(), "https://preserved.example.com");

        let (removed_site, preserved_site) = cache.execute(|conn| {
            let removed_site = SiteRepository
                .select_self_hosted_site_by_url(conn, "https://removed.example.com")
                .expect("removed site lookup should succeed")
                .expect("removed site should exist")
                .data
                .0;
            let preserved_site = SiteRepository
                .select_self_hosted_site_by_url(conn, "https://preserved.example.com")
                .expect("preserved site lookup should succeed")
                .expect("preserved site should exist")
                .data
                .0;
            (removed_site, preserved_site)
        });
        seed_cached_records(&cache, &removed_site);
        seed_cached_records(&cache, &preserved_site);

        assert!(
            service
                .remove_cached_data()
                .expect("cleanup should succeed")
        );
        assert_eq!(
            count_rows_for_site(&cache, "list_metadata", &removed_site),
            0
        );
        assert_eq!(
            count_rows_for_site(&cache, "entity_state", &removed_site),
            0
        );
        assert_eq!(
            count_rows_for_site(&cache, "list_metadata", &preserved_site),
            1
        );
        assert_eq!(
            count_rows_for_site(&cache, "entity_state", &preserved_site),
            1
        );
        cache.execute(|conn| {
            assert!(
                SiteRepository
                    .select_self_hosted_site_by_url(conn, "https://removed.example.com")
                    .expect("removed site lookup should succeed")
                    .is_none()
            );
            assert!(
                SiteRepository
                    .select_self_hosted_site_by_url(conn, "https://preserved.example.com")
                    .expect("preserved site lookup should succeed")
                    .is_some()
            );
        });

        assert!(
            !service
                .remove_cached_data()
                .expect("repeated cleanup should succeed")
        );
        assert!(!foreign_key_enforcement_is_enabled(&cache));
    }

    #[test]
    fn remove_cached_data_deletes_wordpress_com_site() {
        let cache = test_cache();
        cache.execute(|connection| {
            connection
                .pragma_update(None, "foreign_keys", "ON")
                .expect("foreign keys should be enabled for the test");
        });
        let site_id = WpComSiteId(123456);
        let service = wordpress_com_service(cache.clone(), site_id);
        let db_site = cache.execute(|conn| {
            SiteRepository
                .select_wordpress_com_site_by_site_id(conn, site_id)
                .expect("WordPress.com site lookup should succeed")
                .expect("WordPress.com site should exist")
                .data
                .0
        });
        seed_cached_records(&cache, &db_site);

        assert!(
            service
                .remove_cached_data()
                .expect("cleanup should succeed")
        );
        assert_eq!(count_rows_for_site(&cache, "list_metadata", &db_site), 0);
        assert_eq!(count_rows_for_site(&cache, "entity_state", &db_site), 0);
        cache.execute(|conn| {
            assert!(
                SiteRepository
                    .select_wordpress_com_site_by_site_id(conn, site_id)
                    .expect("WordPress.com site lookup should succeed")
                    .is_none()
            );
        });
        assert!(
            !service
                .remove_cached_data()
                .expect("repeated cleanup should succeed")
        );
        assert!(foreign_key_enforcement_is_enabled(&cache));
    }
}
