use std::sync::Arc;
use wp_mobile_cache::{
    SqliteDbError, WpApiCache,
    db_types::db_site::DbSite,
    entity::EntityId,
    repository::sites::SiteRepository,
};

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
}

#[uniffi::export]
impl SiteService {
    /// Get site information (URLs) for the current site
    pub fn get_current_site_info(&self) -> Result<SiteInfo, SqliteDbError> {
        let site_repository = SiteRepository;
        let entity_id = EntityId {
            db_site: self.db_site,
            table_name: SiteRepository::SELF_HOSTED_SITES_TABLE,
            rowid: self.db_site.mapped_site_id,
        };

        self.cache.execute(|conn| {
            site_repository
                .select_self_hosted_site(conn, &entity_id)
                .map(|opt| {
                    opt.map(|full_entity| SiteInfo {
                        site_url: full_entity.data.url,
                        api_root: full_entity.data.api_root,
                    })
                    .unwrap_or_else(|| SiteInfo {
                        site_url: String::new(),
                        api_root: String::new(),
                    })
                })
        })
    }
}
