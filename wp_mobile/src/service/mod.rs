use crate::service::{post_types::PostTypeService, posts::PostService, sites::SiteService};
use std::sync::Arc;
use url::Url;
use wp_api::prelude::{
    ApiUrlResolver, ParsedUrl, WpApiClient, WpApiClientDelegate, WpOrgSiteApiUrlResolver,
};
use wp_api::wp_com::{WpComBaseUrl, WpComSiteId, endpoint::WpComDotOrgApiUrlResolver};
use wp_mobile_cache::{WpApiCache, db_types::db_site::DbSite};

const WPCOM_API_HOST: &str = "public-api.wordpress.com";

/// Returns the canonical API root URL for a WordPress.com site.
///
/// This URL can be stored as the `api_root` for a self-hosted site account.
/// When passed to `WpService.selfHosted()`, it will automatically use
/// WordPress.com URL rewriting.
#[uniffi::export]
pub fn wordpress_com_site_api_root(site_id: u64) -> String {
    format!("https://{WPCOM_API_HOST}/wp/v2/sites/{site_id}")
}

/// Extracts the WordPress.com site ID from an API root URL, if it matches
/// the `https://public-api.wordpress.com/wp/v2/sites/{site_id}` pattern.
fn extract_wpcom_site_id(api_root: &str) -> Option<WpComSiteId> {
    let url = Url::parse(api_root).ok()?;
    if url.host_str()? != WPCOM_API_HOST {
        return None;
    }
    let segments: Vec<&str> = url.path_segments()?.collect();
    if segments.len() >= 4 && segments[0] == "wp" && segments[1] == "v2" && segments[2] == "sites" {
        segments[3].parse::<u64>().ok().map(WpComSiteId)
    } else {
        None
    }
}

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
    /// If the `api_root` is a WordPress.com API root URL (as produced by
    /// `wordpress_com_site_api_root()`), this constructor will automatically
    /// use WordPress.com URL rewriting and store the site as a WordPress.com site.
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
        if let Some(site_id) = extract_wpcom_site_id(&api_root) {
            return Self::new_wordpress_com(site_id, delegate, cache);
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wordpress_com_site_api_root() {
        assert_eq!(
            wordpress_com_site_api_root(12345),
            "https://public-api.wordpress.com/wp/v2/sites/12345"
        );
    }

    #[test]
    fn test_extract_wpcom_site_id_valid() {
        let url = "https://public-api.wordpress.com/wp/v2/sites/12345";
        assert_eq!(extract_wpcom_site_id(url), Some(WpComSiteId(12345)));
    }

    #[test]
    fn test_extract_wpcom_site_id_roundtrip() {
        let url = wordpress_com_site_api_root(67890);
        assert_eq!(extract_wpcom_site_id(&url), Some(WpComSiteId(67890)));
    }

    #[test]
    fn test_extract_wpcom_site_id_self_hosted() {
        assert_eq!(extract_wpcom_site_id("https://example.com/wp-json"), None);
    }

    #[test]
    fn test_extract_wpcom_site_id_wrong_path() {
        assert_eq!(
            extract_wpcom_site_id("https://public-api.wordpress.com/rest/v1.1/sites/12345"),
            None
        );
    }

    #[test]
    fn test_extract_wpcom_site_id_invalid_url() {
        assert_eq!(extract_wpcom_site_id("not a url"), None);
    }
}
