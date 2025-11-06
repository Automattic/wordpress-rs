use crate::service::posts::PostService;
use std::sync::Arc;
use wp_api::prelude::{ApiUrlResolver, WpApiClient, WpApiClientDelegate};
use wp_mobile_cache::{
    WpApiCache,
    db_types::{db_site::DbSite, self_hosted_site::SelfHostedSite},
    repository::sites::SiteRepository,
};

pub mod posts;

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
}

impl WpSelfHostedService {
    /// Look up or create the DbSite for this service
    fn get_or_create_db_site(
        api_url_resolver: &Arc<dyn ApiUrlResolver>,
        cache: &WpApiCache,
    ) -> Result<DbSite, WpServiceError> {
        let site_repository = SiteRepository;

        // Get site URL from resolver
        let site_url_parsed = api_url_resolver.resolve("".to_string(), vec![]);
        let site_url = site_url_parsed.as_str();

        // First, try to find existing site
        {
            let connection = cache.connection();
            if let Some((db_site, _)) =
                site_repository.select_self_hosted_site_by_url(&*connection, site_url)?
            {
                return Ok(db_site);
            }
        } // Drop the connection guard here

        // Site doesn't exist, create it
        let mut connection = cache.connection();

        // Get API root from resolver
        let api_root_parsed = api_url_resolver.resolve("".to_string(), vec![]);

        let self_hosted_site = SelfHostedSite {
            url: site_url.to_string(),
            api_root: api_root_parsed.as_str().to_string(),
        };

        let (db_site, _) =
            site_repository.upsert_self_hosted_site(&mut *connection, &self_hosted_site)?;

        Ok(db_site)
    }
}

#[uniffi::export]
impl WpSelfHostedService {
    /// Create a new service for a self-hosted WordPress site
    ///
    /// This will look up the site in the cache or create it if it doesn't exist.
    #[uniffi::constructor]
    pub fn new(
        api_url_resolver: Arc<dyn ApiUrlResolver>,
        delegate: WpApiClientDelegate,
        cache: Arc<WpApiCache>,
    ) -> Result<Self, WpServiceError> {
        let api_client = Arc::new(WpApiClient::new(api_url_resolver.clone(), delegate));
        let db_site = Arc::new(Self::get_or_create_db_site(&api_url_resolver, &cache)?);

        Ok(Self {
            posts: Arc::new(PostService::new(api_client, db_site, cache)),
        })
    }

    pub fn posts(&self) -> Arc<PostService> {
        self.posts.clone()
    }
}
