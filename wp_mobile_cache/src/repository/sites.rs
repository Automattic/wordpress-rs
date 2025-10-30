use crate::{
    DbSite, DbSiteType, RowId, SqliteDbError,
    db_types::{
        row_ext::RowExt,
        self_hosted_site::{DbSelfHostedSite, SelfHostedSite, SelfHostedSiteColumn},
    },
    repository::QueryExecutor,
};
use rusqlite::OptionalExtension;

pub struct SiteRepository;

impl SiteRepository {
    const SELF_HOSTED_SITES_TABLE: &'static str = "self_hosted_sites";
    const SITES_TABLE: &'static str = "sites";

    /// Upsert a self-hosted site and return both the DbSite and DbSelfHostedSite.
    ///
    /// If a site with the given URL already exists, updates it. Otherwise creates a new one.
    /// Uses SQLite's RETURNING clause to get the inserted/updated rowid.
    pub fn upsert_self_hosted_site(
        &self,
        executor: &impl QueryExecutor,
        site: &SelfHostedSite,
    ) -> Result<(DbSite, DbSelfHostedSite), SqliteDbError> {
        // Upsert into self_hosted_sites and get the rowid
        let sql = format!(
            "INSERT INTO {} (url, api_root) VALUES (?, ?)
             ON CONFLICT(url) DO UPDATE SET api_root = excluded.api_root
             RETURNING rowid",
            Self::SELF_HOSTED_SITES_TABLE
        );

        let mut stmt = executor.prepare(&sql)?;
        let self_hosted_site_id: RowId = stmt
            .query_row((&site.url, &site.api_root), |row| row.get(0))
            .map_err(SqliteDbError::from)?;

        // Insert into sites table
        let sql = format!(
            "INSERT INTO {} (site_type, mapped_site_id) VALUES (?, ?)
             RETURNING rowid",
            Self::SITES_TABLE
        );

        let mut stmt = executor.prepare(&sql)?;
        let site_id: RowId = stmt
            .query_row((DbSiteType::SelfHosted, self_hosted_site_id), |row| {
                row.get(0)
            })
            .map_err(SqliteDbError::from)?;

        let db_site = DbSite {
            row_id: site_id,
            site_type: DbSiteType::SelfHosted,
            mapped_site_id: self_hosted_site_id,
        };

        let db_self_hosted_site = DbSelfHostedSite {
            row_id: self_hosted_site_id,
            url: site.url.clone(),
            api_root: site.api_root.clone(),
        };

        Ok((db_site, db_self_hosted_site))
    }

    /// Select a self-hosted site by its DbSite reference.
    ///
    /// Returns None if the site doesn't exist or isn't a self-hosted site.
    pub fn select_self_hosted_site(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
    ) -> Result<Option<DbSelfHostedSite>, SqliteDbError> {
        if site.site_type != DbSiteType::SelfHosted {
            return Ok(None);
        }

        let sql = format!(
            "SELECT * FROM {} WHERE rowid = ?",
            Self::SELF_HOSTED_SITES_TABLE
        );
        let mut stmt = executor.prepare(&sql)?;

        stmt.query_row([site.mapped_site_id], |row| {
            Ok(DbSelfHostedSite {
                row_id: row.get_column(SelfHostedSiteColumn::Rowid)?,
                url: row.get_column(SelfHostedSiteColumn::Url)?,
                api_root: row.get_column(SelfHostedSiteColumn::ApiRoot)?,
            })
        })
        .optional()
        .map_err(SqliteDbError::from)
    }

    /// Select a self-hosted site by URL.
    ///
    /// Returns both the DbSite and DbSelfHostedSite if found.
    pub fn select_self_hosted_site_by_url(
        &self,
        executor: &impl QueryExecutor,
        url: &str,
    ) -> Result<Option<(DbSite, DbSelfHostedSite)>, SqliteDbError> {
        // First get the self_hosted_site
        let sql = format!(
            "SELECT * FROM {} WHERE url = ?",
            Self::SELF_HOSTED_SITES_TABLE
        );
        let mut stmt = executor.prepare(&sql)?;

        let self_hosted_site: Option<DbSelfHostedSite> = stmt
            .query_row([url], |row| {
                Ok(DbSelfHostedSite {
                    row_id: row.get_column(SelfHostedSiteColumn::Rowid)?,
                    url: row.get_column(SelfHostedSiteColumn::Url)?,
                    api_root: row.get_column(SelfHostedSiteColumn::ApiRoot)?,
                })
            })
            .optional()
            .map_err(SqliteDbError::from)?;

        let Some(self_hosted_site) = self_hosted_site else {
            return Ok(None);
        };

        // Then find the corresponding site
        let sql = format!(
            "SELECT rowid, site_type, mapped_site_id FROM {}
             WHERE site_type = ? AND mapped_site_id = ?",
            Self::SITES_TABLE
        );
        let mut stmt = executor.prepare(&sql)?;

        let db_site: Option<DbSite> = stmt
            .query_row((DbSiteType::SelfHosted, self_hosted_site.row_id), |row| {
                Ok(DbSite {
                    row_id: row.get(0)?,
                    site_type: row.get(1)?,
                    mapped_site_id: row.get(2)?,
                })
            })
            .optional()
            .map_err(SqliteDbError::from)?;

        Ok(db_site.map(|site| (site, self_hosted_site)))
    }
}
