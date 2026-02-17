use crate::service::{post_types::PostTypeService, posts::PostService, sites::SiteService};
use std::sync::Arc;
use wp_api::prelude::{
    ApiUrlResolver, ParsedUrl, WpApiClient, WpApiClientDelegate, WpOrgSiteApiUrlResolver,
};
use wp_api::wp_com::{WpComBaseUrl, WpComSiteId, endpoint::WpComDotOrgApiUrlResolver};
use wp_mobile_cache::{WpApiCache, db_types::db_site::DbSite};

pub mod entity_state_service;
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
    posts: Arc<PostService>,
    post_types: Arc<PostTypeService>,
    sites: Arc<SiteService>,
}

impl WpService {
    fn build_services(
        api_url_resolver: Arc<dyn ApiUrlResolver>,
        delegate: WpApiClientDelegate,
        cache: Arc<WpApiCache>,
        db_site: DbSite,
        site_service: Arc<SiteService>,
    ) -> Self {
        let api_client = Arc::new(WpApiClient::new(api_url_resolver, delegate));
        let db_site_arc = Arc::new(db_site);

        let posts = Arc::new(PostService::new(
            api_client.clone(),
            db_site_arc.clone(),
            cache.clone(),
        ));
        let post_types = Arc::new(PostTypeService::new(api_client, db_site_arc, cache));

        Self {
            posts,
            post_types,
            sites: site_service,
        }
    }
}

#[uniffi::export]
impl WpService {
    /// Create a new service for a self-hosted WordPress site
    ///
    /// This will look up the site in the cache or create it if it doesn't exist.
    ///
    /// # Arguments
    /// * `site_url` - The base site URL (e.g., "https://example.com")
    /// * `api_root` - The API root URL (e.g., "https://example.com/wp-json")
    /// * `delegate` - API client delegate with auth provider, request executor, etc.
    /// * `cache` - The cache instance for database operations
    #[uniffi::constructor(name = "selfHosted")]
    pub fn new_self_hosted(
        site_url: String,
        api_root: String,
        delegate: WpApiClientDelegate,
        cache: Arc<WpApiCache>,
    ) -> Result<Self, WpServiceError> {
        let api_root_url = ParsedUrl::parse(&api_root).map_err(|e| WpServiceError::InvalidUrl {
            err_message: e.to_string(),
        })?;
        let api_url_resolver: Arc<dyn ApiUrlResolver> =
            Arc::new(WpOrgSiteApiUrlResolver::new(Arc::new(api_root_url)));
        let db_site =
            SiteService::get_or_create_self_hosted_site(cache.clone(), site_url, api_root)?;
        let sites = Arc::new(SiteService::new(cache.clone(), db_site));
        Ok(Self::build_services(
            api_url_resolver,
            delegate,
            cache,
            db_site,
            sites,
        ))
    }

    /// Create a new service for a WordPress.com site
    ///
    /// This will look up the site in the cache or create it if it doesn't exist.
    ///
    /// # Arguments
    /// * `site_id` - The WordPress.com site ID
    /// * `delegate` - API client delegate with auth provider, request executor, etc.
    /// * `cache` - The cache instance for database operations
    #[uniffi::constructor(name = "wordpressCom")]
    pub fn new_wordpress_com(
        site_id: WpComSiteId,
        delegate: WpApiClientDelegate,
        cache: Arc<WpApiCache>,
    ) -> Result<Self, WpServiceError> {
        let api_url_resolver: Arc<dyn ApiUrlResolver> = Arc::new(WpComDotOrgApiUrlResolver::new(
            site_id.to_string(),
            WpComBaseUrl::default(),
        ));
        let db_site = SiteService::get_or_create_wordpress_com_site(cache.clone(), site_id)?;
        let sites = Arc::new(SiteService::new(cache.clone(), db_site));
        Ok(Self::build_services(
            api_url_resolver,
            delegate,
            cache,
            db_site,
            sites,
        ))
    }

    /// Get the post service for this WordPress site
    pub fn posts(&self) -> Arc<PostService> {
        self.posts.clone()
    }

    /// Get the post type service for this WordPress site
    pub fn post_types(&self) -> Arc<PostTypeService> {
        self.post_types.clone()
    }

    /// Get the site service for this WordPress site
    pub fn sites(&self) -> Arc<SiteService> {
        self.sites.clone()
    }
}
