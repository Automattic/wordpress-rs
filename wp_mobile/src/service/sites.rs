use std::sync::Arc;
use wp_mobile_cache::{
    DbTable, SqliteDbError, WpApiCache, db_types::db_site::DbSite,
    db_types::self_hosted_site::SelfHostedSite, entity::EntityId,
    repository::sites::SiteRepository,
};
use crate::service::WpServiceError;

/// Information about a site's URLs
#[derive(Debug, Clone, uniffi::Record)]
pub struct SiteInfo {
    pub site_url: String,
    pub api_root: String,
}

/// Service for site-related operations
#[derive(uniffi::Object)]
pub struct SiteService {
    cache: Arc<WpApiCache>,
    db_site: DbSite,
}

impl SiteService {
    pub fn new(cache: Arc<WpApiCache>, db_site: DbSite) -> Self {
        Self { cache, db_site }
    }

    /// Get or create a DbSite for a self-hosted WordPress site
    ///
    /// Looks up an existing site by URL, or creates it if not found.
    /// This is an internal helper called by WpSelfHostedService::new().
    ///
    /// # Arguments
    /// * `cache` - The cache instance to use for database operations
    /// * `site_url` - The base site URL (e.g., "https://example.com")
    /// * `api_root` - The API root URL (e.g., "https://example.com/wp-json")
    ///
    /// # Returns
    /// The DbSite for the site, either existing or newly created
    pub(crate) fn get_or_create_self_hosted_site(
        cache: Arc<WpApiCache>,
        site_url: String,
        api_root: String,
    ) -> Result<DbSite, SqliteDbError> {
        let site_repository = SiteRepository;

        cache.execute(|conn| {
            // Try to find existing site by URL
            if let Some(full_entity) =
                site_repository.select_self_hosted_site_by_url(conn, &site_url)?
            {
                return Ok(full_entity.data.0);
            }

            // Site doesn't exist, create it
            let self_hosted_site = SelfHostedSite {
                url: site_url,
                api_root,
            };

            let entity_id = site_repository.upsert_self_hosted_site(conn, &self_hosted_site)?;
            Ok(entity_id.db_site)
        })
    }
}

#[uniffi::export]
impl SiteService {
    /// Get site information (URLs) for the current site
    ///
    /// Returns the site URL and API root for the site associated with this service.
    /// This is useful when you need to display or use the site's URLs in the UI or
    /// for other services that need to work with the same site.
    ///
    /// # Returns
    /// - `Ok(SiteInfo)` with the site's URLs
    /// - `Err(WpServiceError::DatabaseError)` if a database error occurs
    /// - `Err(WpServiceError::SiteNotFound)` if the site doesn't exist in the database
    ///
    /// # Note
    /// Since the `SiteService` is constructed with a valid `DbSite`, the site should
    /// normally exist in the database. A `SiteNotFound` error indicates a data
    /// inconsistency (e.g., site was deleted but entities still reference it).
    pub fn get_current_site_info(&self) -> Result<SiteInfo, WpServiceError> {
        let site_repository = SiteRepository;
        let entity_id = EntityId {
            db_site: self.db_site,
            table: DbTable::SelfHostedSites,
            rowid: self.db_site.mapped_site_id,
        };

        let full_entity = self
            .cache
            .execute(|conn| site_repository.select_self_hosted_site(conn, &entity_id))?;

        full_entity
            .ok_or(WpServiceError::SiteNotFound)
            .map(|entity| SiteInfo {
                site_url: entity.data.url,
                api_root: entity.data.api_root,
            })
    }
}
