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

    /// Get the request executor used by this service.
    pub fn request_executor(&self) -> Arc<dyn RequestExecutor> {
        self.request_executor.clone()
    }
}
