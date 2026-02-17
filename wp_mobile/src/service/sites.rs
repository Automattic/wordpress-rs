use crate::service::WpServiceError;
use std::sync::Arc;
use wp_api::wp_com::WpComSiteId;
use wp_mobile_cache::{
    DbTable, SqliteDbError, WpApiCache,
    db_types::db_site::{DbSite, DbSiteType},
    db_types::self_hosted_site::SelfHostedSite,
    db_types::wordpress_com_site::WordPressComSite,
    entity::EntityId,
    repository::sites::SiteRepository,
};

/// Information about a site
#[derive(Debug, Clone, uniffi::Enum)]
pub enum SiteInfo {
    SelfHosted { site_url: String, api_root: String },
    WordPressCom { site_id: WpComSiteId },
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
    /// This is an internal helper called by WpService::new_self_hosted().
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

    /// Get or create a DbSite for a WordPress.com site
    ///
    /// Looks up an existing site by site_id, or creates it if not found.
    /// This is an internal helper called by WpService::new_wordpress_com().
    pub(crate) fn get_or_create_wordpress_com_site(
        cache: Arc<WpApiCache>,
        site_id: WpComSiteId,
    ) -> Result<DbSite, SqliteDbError> {
        let site_repository = SiteRepository;

        cache.execute(|conn| {
            if let Some(full_entity) =
                site_repository.select_wordpress_com_site_by_site_id(conn, site_id)?
            {
                return Ok(full_entity.data.0);
            }

            let wp_com_site = WordPressComSite { site_id };

            let entity_id = site_repository.upsert_wordpress_com_site(conn, &wp_com_site)?;
            Ok(entity_id.db_site)
        })
    }
}

#[uniffi::export]
impl SiteService {
    /// Get site information for the current site
    ///
    /// Returns the site information variant depending on the site type.
    ///
    /// # Returns
    /// - `Ok(SiteInfo::SelfHosted { .. })` for self-hosted sites
    /// - `Ok(SiteInfo::WordPressCom { .. })` for WordPress.com sites
    /// - `Err(WpServiceError::DatabaseError)` if a database error occurs
    /// - `Err(WpServiceError::SiteNotFound)` if the site doesn't exist in the database
    pub fn get_current_site_info(&self) -> Result<SiteInfo, WpServiceError> {
        match self.db_site.site_type {
            DbSiteType::SelfHosted => {
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
                    .map(|entity| SiteInfo::SelfHosted {
                        site_url: entity.data.url,
                        api_root: entity.data.api_root,
                    })
            }
            DbSiteType::WordPressCom => {
                let site_repository = SiteRepository;
                let entity_id = EntityId {
                    db_site: self.db_site,
                    table: DbTable::WordPressComSites,
                    rowid: self.db_site.mapped_site_id,
                };

                let full_entity = self
                    .cache
                    .execute(|conn| site_repository.select_wordpress_com_site(conn, &entity_id))?;

                full_entity
                    .ok_or(WpServiceError::SiteNotFound)
                    .map(|entity| SiteInfo::WordPressCom {
                        site_id: entity.data.site_id,
                    })
            }
        }
    }
}
