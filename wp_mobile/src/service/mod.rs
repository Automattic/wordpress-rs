use crate::service::{posts::PostService, sites::SiteService};
use std::sync::Arc;
use wp_api::prelude::{ApiUrlResolver, WpApiClient, WpApiClientDelegate};
use wp_mobile_cache::WpApiCache;

pub mod mock_post_service;
pub mod posts;
pub mod sites;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum WpServiceError {
    #[error("Database error: {err_message}")]
    DatabaseError { err_message: String },

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

/// Service for self-hosted WordPress sites
///
/// This service coordinates between the API client and cache for a specific
/// self-hosted WordPress site. It provides access to domain-specific services
/// like PostService, CommentService, etc.
#[derive(uniffi::Object)]
pub struct WpSelfHostedService {
    posts: Arc<PostService>,
    sites: Arc<SiteService>,
}

#[uniffi::export]
impl WpSelfHostedService {
    /// Create a new service for a self-hosted WordPress site
    ///
    /// This will look up the site in the cache or create it if it doesn't exist.
    ///
    /// # Arguments
    /// * `site_url` - The base site URL (e.g., "https://example.com")
    /// * `api_root` - The API root URL (e.g., "https://example.com/wp-json")
    /// * `api_url_resolver` - URL resolver for building API endpoint URLs
    /// * `delegate` - API client delegate with auth provider, request executor, etc.
    /// * `cache` - The cache instance for database operations
    #[uniffi::constructor]
    pub fn new(
        site_url: String,
        api_root: String,
        api_url_resolver: Arc<dyn ApiUrlResolver>,
        delegate: WpApiClientDelegate,
        cache: Arc<WpApiCache>,
    ) -> Result<Self, WpServiceError> {
        let api_client = Arc::new(WpApiClient::new(api_url_resolver, delegate));

        // Get or create the DbSite
        let db_site =
            SiteService::get_or_create_self_hosted_site(cache.clone(), site_url, api_root)?;

        let posts = Arc::new(PostService::new(
            api_client,
            Arc::new(db_site),
            cache.clone(),
        ));
        let sites = Arc::new(SiteService::new(cache, db_site));

        Ok(Self { posts, sites })
    }

    /// Get the post service for this WordPress site
    pub fn posts(&self) -> Arc<PostService> {
        self.posts.clone()
    }

    /// Get the site service for this WordPress site
    pub fn sites(&self) -> Arc<SiteService> {
        self.sites.clone()
    }
}
