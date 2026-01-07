use crate::{
    DbTable, RowId, SqliteDbError,
    db_types::{
        db_site::{DbSite, DbSiteType},
        self_hosted_site::{DbSelfHostedSite, SelfHostedSite},
    },
    entity::{EntityId, FullEntity},
    repository::{QueryExecutor, TransactionManager},
};
use rusqlite::OptionalExtension;
use std::sync::Arc;

pub struct SiteRepository;

impl SiteRepository {
    /// Upsert a self-hosted site and return its EntityId (atomic transaction).
    ///
    /// If a site with the given URL already exists, updates it. Otherwise creates a new one.
    /// Uses SQLite's RETURNING clause to get the inserted/updated rowid.
    pub fn upsert_self_hosted_site(
        &self,
        transaction_manager: &mut impl TransactionManager,
        site: &SelfHostedSite,
    ) -> Result<EntityId, SqliteDbError> {
        let tx = transaction_manager.transaction()?;

        // Upsert into self_hosted_sites and get the rowid
        let self_hosted_site_id: RowId = {
            let sql = format!(
                "INSERT INTO {} (url, api_root) VALUES (?, ?)
                 ON CONFLICT(url) DO UPDATE SET api_root = excluded.api_root
                 RETURNING rowid",
                DbTable::SelfHostedSites.table_name()
            );

            let mut stmt = tx.prepare(&sql)?;
            stmt.query_row((&site.url, &site.api_root), |row| row.get(0))
                .map_err(SqliteDbError::from)?
        };

        // Upsert into sites table
        // If site_type + mapped_site_id already exists, reuse that entry
        // Note: DO UPDATE SET is required for RETURNING to work on conflict (DO NOTHING returns no rows)
        let site_id: RowId = {
            let sql = format!(
                "INSERT INTO {} (site_type, mapped_site_id) VALUES (?, ?)
                 ON CONFLICT(site_type, mapped_site_id) DO UPDATE SET
                    site_type = excluded.site_type
                 RETURNING rowid",
                DbTable::DbSites.table_name()
            );

            let mut stmt = tx.prepare(&sql)?;
            stmt.query_row((DbSiteType::SelfHosted, self_hosted_site_id), |row| {
                row.get(0)
            })
            .map_err(SqliteDbError::from)?
        };

        let db_site = DbSite {
            row_id: site_id,
            site_type: DbSiteType::SelfHosted,
            mapped_site_id: self_hosted_site_id,
        };

        tx.commit().map_err(SqliteDbError::from)?;
        Ok(EntityId::new(
            db_site,
            DbTable::SelfHostedSites,
            self_hosted_site_id,
        ))
    }

    /// Select a self-hosted site by its EntityId.
    ///
    /// Returns the site data paired with its EntityId, which encapsulates the
    /// database identity (db_site_id, table_name, rowid).
    ///
    /// Returns an error if the EntityId's table name doesn't match "self_hosted_sites".
    /// Returns None if the site doesn't exist or isn't a self-hosted site.
    pub fn select_self_hosted_site(
        &self,
        executor: &impl QueryExecutor,
        entity_id: &EntityId,
    ) -> Result<Option<FullEntity<DbSelfHostedSite>>, SqliteDbError> {
        // Validate that the entity_id is for the correct table
        entity_id.validate_table(DbTable::SelfHostedSites)?;

        if entity_id.db_site.site_type != DbSiteType::SelfHosted {
            return Ok(None);
        }

        let sql = format!(
            "SELECT * FROM {} WHERE rowid = ?",
            DbTable::SelfHostedSites.table_name()
        );
        let mut stmt = executor.prepare(&sql)?;

        let db_self_hosted_site = stmt
            .query_row([entity_id.db_site.mapped_site_id], |row| {
                Ok(DbSelfHostedSite {
                    row_id: row.get("rowid")?,
                    url: row.get("url")?,
                    api_root: row.get("api_root")?,
                })
            })
            .optional()
            .map_err(SqliteDbError::from)?;

        Ok(db_self_hosted_site.map(|db_self_hosted_site| {
            let entity_id = Arc::new(*entity_id);
            FullEntity::new(entity_id, db_self_hosted_site)
        }))
    }

    /// Select a self-hosted site by URL.
    ///
    /// Returns the site data (DbSite and DbSelfHostedSite) paired with its EntityId,
    /// which encapsulates the database identity (db_site_id, table_name, rowid).
    pub fn select_self_hosted_site_by_url(
        &self,
        executor: &impl QueryExecutor,
        url: &str,
    ) -> Result<Option<FullEntity<(DbSite, DbSelfHostedSite)>>, SqliteDbError> {
        // First get the self_hosted_site
        let sql = format!(
            "SELECT * FROM {} WHERE url = ?",
            DbTable::SelfHostedSites.table_name()
        );
        let mut stmt = executor.prepare(&sql)?;

        let self_hosted_site: Option<DbSelfHostedSite> = stmt
            .query_row([url], |row| {
                Ok(DbSelfHostedSite {
                    row_id: row.get("rowid")?,
                    url: row.get("url")?,
                    api_root: row.get("api_root")?,
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
            DbTable::DbSites.table_name()
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

        Ok(db_site.map(|db_site| {
            let entity_id = Arc::new(EntityId::new(
                db_site,
                DbTable::SelfHostedSites,
                self_hosted_site.row_id,
            ));
            FullEntity::new(entity_id, (db_site, self_hosted_site))
        }))
    }

    /// Count all sites in the database.
    ///
    /// This is primarily useful for testing to verify database state.
    pub fn count_all_db_sites(
        &self,
        executor: &impl QueryExecutor,
    ) -> Result<usize, SqliteDbError> {
        let sql = format!("SELECT COUNT(*) FROM {}", DbTable::DbSites.table_name());
        let mut stmt = executor.prepare(&sql)?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Count all self-hosted sites in the database.
    ///
    /// This is primarily useful for testing to verify database state.
    pub fn count_all_self_hosted_sites(
        &self,
        executor: &impl QueryExecutor,
    ) -> Result<usize, SqliteDbError> {
        let sql = format!(
            "SELECT COUNT(*) FROM {}",
            DbTable::SelfHostedSites.table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Delete a site and its type-specific data (atomic transaction).
    ///
    /// Deletes both the site entry and its corresponding type-specific entry
    /// (e.g., from self_hosted_sites or wordpress_com_sites). This ensures proper
    /// cleanup since foreign key constraints cannot be used with polymorphic references.
    ///
    /// Returns `true` if a site was deleted, `false` if the site didn't exist.
    pub fn delete_site(
        &self,
        transaction_manager: &mut impl TransactionManager,
        site: &DbSite,
    ) -> Result<bool, SqliteDbError> {
        let tx = transaction_manager.transaction()?;

        // Delete from type-specific table based on site_type
        match site.site_type {
            DbSiteType::SelfHosted => {
                let sql = format!(
                    "DELETE FROM {} WHERE rowid = ?",
                    DbTable::SelfHostedSites.table_name()
                );
                tx.execute(&sql, [site.mapped_site_id])?;
            }
            DbSiteType::WordPressCom => {
                panic!("WordPress.com site deletion is not yet implemented")
            }
        };

        // Delete from sites table
        let sites_deleted = {
            let sql = format!(
                "DELETE FROM {} WHERE rowid = ?",
                DbTable::DbSites.table_name()
            );
            tx.execute(&sql, [site.row_id])?
        };

        tx.commit().map_err(SqliteDbError::from)?;
        Ok(sites_deleted > 0)
    }

    /// Delete a self-hosted site by URL (convenience wrapper).
    ///
    /// Returns `true` if a site was deleted, `false` if no site with that URL exists.
    pub fn delete_self_hosted_site_by_url(
        &self,
        transaction_manager: &mut impl TransactionManager,
        url: &str,
    ) -> Result<bool, SqliteDbError> {
        let site_data = self.select_self_hosted_site_by_url(transaction_manager, url)?;

        match site_data {
            Some(full_entity) => self.delete_site(transaction_manager, &full_entity.data.0),
            None => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MigrationManager, entity::EntityId};
    use rstest::*;
    use rusqlite::Connection;

    #[fixture]
    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory database");
        let mut migration_manager =
            MigrationManager::new(&conn).expect("Failed to create MigrationManager");
        migration_manager
            .perform_migrations()
            .expect("All migrations should succeed");
        conn
    }

    #[rstest]
    fn test_upsert_inserts_new_site(mut test_conn: Connection) {
        let repo = SiteRepository;
        let site = SelfHostedSite {
            url: "https://example.com".to_string(),
            api_root: "https://example.com/wp-json".to_string(),
        };

        let entity_id = repo
            .upsert_self_hosted_site(&mut test_conn, &site)
            .expect("Failed to upsert site");

        // Verify the EntityId has correct table and site type
        assert_eq!(entity_id.table, DbTable::SelfHostedSites);
        assert_eq!(entity_id.db_site.site_type, DbSiteType::SelfHosted);
        assert_eq!(entity_id.db_site.mapped_site_id, entity_id.rowid);
    }

    #[rstest]
    fn test_upsert_updates_existing_site_url(mut test_conn: Connection) {
        let repo = SiteRepository;
        let url = "https://example.com";

        // First insert
        let site1 = SelfHostedSite {
            url: url.to_string(),
            api_root: "https://example.com/wp-json".to_string(),
        };
        let entity_id1 = repo
            .upsert_self_hosted_site(&mut test_conn, &site1)
            .expect("Failed to upsert site");

        // Second upsert with same URL but different api_root
        let site2 = SelfHostedSite {
            url: url.to_string(),
            api_root: "https://example.com/wordpress/wp-json".to_string(),
        };
        let entity_id2 = repo
            .upsert_self_hosted_site(&mut test_conn, &site2)
            .expect("Failed to upsert site");

        // Verify it updated the same row (same EntityId) - this proves update, not insert
        assert_eq!(entity_id1, entity_id2);
    }

    #[rstest]
    fn test_upsert_does_not_create_duplicate_sites_entries(mut test_conn: Connection) {
        let repo = SiteRepository;

        // Insert same site multiple times
        let site = SelfHostedSite {
            url: "https://example.com".to_string(),
            api_root: "https://example.com/wp-json".to_string(),
        };

        let entity_id1 = repo
            .upsert_self_hosted_site(&mut test_conn, &site)
            .expect("First upsert failed");
        let entity_id2 = repo
            .upsert_self_hosted_site(&mut test_conn, &site)
            .expect("Second upsert failed");
        let entity_id3 = repo
            .upsert_self_hosted_site(&mut test_conn, &site)
            .expect("Third upsert failed");

        // All should return the same EntityId
        assert_eq!(entity_id1, entity_id2);
        assert_eq!(entity_id2, entity_id3);

        // Verify only one entry exists in sites table (ensures bug fix works)
        let count = repo
            .count_all_db_sites(&test_conn)
            .expect("Failed to count db_sites");
        assert_eq!(
            count, 1,
            "Multiple upserts should not create duplicate sites table entries"
        );
    }

    #[rstest]
    fn test_select_self_hosted_site_by_db_site(mut test_conn: Connection) {
        let repo = SiteRepository;
        let site = SelfHostedSite {
            url: "https://example.com".to_string(),
            api_root: "https://example.com/wp-json".to_string(),
        };

        let entity_id = repo
            .upsert_self_hosted_site(&mut test_conn, &site)
            .expect("Failed to upsert site");

        // Select using EntityId
        let retrieved = repo
            .select_self_hosted_site(&test_conn, &entity_id)
            .expect("Failed to select site")
            .expect("Site should exist");

        assert_eq!(retrieved.data.url, site.url);
        assert_eq!(retrieved.data.api_root, site.api_root);
    }

    #[rstest]
    fn test_select_self_hosted_site_returns_none_for_wrong_site_type(test_conn: Connection) {
        let repo = SiteRepository;

        // Create a DbSite with WordPressCom type (not inserted, just for testing)
        let non_self_hosted_site = DbSite {
            row_id: RowId(999),
            site_type: DbSiteType::WordPressCom,
            mapped_site_id: RowId(999),
        };

        // Create an EntityId for this non-self-hosted site
        let entity_id = EntityId::new(non_self_hosted_site, DbTable::SelfHostedSites, RowId(999));

        let result = repo
            .select_self_hosted_site(&test_conn, &entity_id)
            .expect("Query should succeed");

        assert_eq!(
            result, None,
            "Should return None for non-SelfHosted site type"
        );
    }

    #[rstest]
    fn test_select_self_hosted_site_returns_none_for_non_existent_site(test_conn: Connection) {
        let repo = SiteRepository;

        let non_existent_site = DbSite {
            row_id: RowId(999),
            site_type: DbSiteType::SelfHosted,
            mapped_site_id: RowId(999),
        };

        // Create an EntityId for this non-existent site
        let entity_id = EntityId::new(non_existent_site, DbTable::SelfHostedSites, RowId(999));

        let result = repo
            .select_self_hosted_site(&test_conn, &entity_id)
            .expect("Query should succeed");

        assert_eq!(result, None, "Should return None for non-existent site");
    }

    #[rstest]
    fn test_select_self_hosted_site_by_url(mut test_conn: Connection) {
        let repo = SiteRepository;
        let site = SelfHostedSite {
            url: "https://example.com".to_string(),
            api_root: "https://example.com/wp-json".to_string(),
        };

        let entity_id = repo
            .upsert_self_hosted_site(&mut test_conn, &site)
            .expect("Failed to upsert site");

        // Select by URL
        let retrieved = repo
            .select_self_hosted_site_by_url(&test_conn, &site.url)
            .expect("Failed to select site")
            .expect("Site should exist");

        // Verify the EntityId matches
        assert_eq!(retrieved.entity_id.as_ref(), &entity_id);
        // Verify data matches
        assert_eq!(retrieved.data.0, entity_id.db_site);
        assert_eq!(retrieved.data.1.url, site.url);
        assert_eq!(retrieved.data.1.api_root, site.api_root);
    }

    #[rstest]
    fn test_select_self_hosted_site_by_url_returns_none_for_non_existent_url(
        test_conn: Connection,
    ) {
        let repo = SiteRepository;

        let result = repo
            .select_self_hosted_site_by_url(&test_conn, "https://non-existent.com")
            .expect("Query should succeed");

        assert_eq!(result, None, "Should return None for non-existent URL");
    }

    #[rstest]
    fn test_multiple_different_sites_can_coexist(mut test_conn: Connection) {
        let repo = SiteRepository;

        let site1 = SelfHostedSite {
            url: "https://example1.com".to_string(),
            api_root: "https://example1.com/wp-json".to_string(),
        };
        let site2 = SelfHostedSite {
            url: "https://example2.com".to_string(),
            api_root: "https://example2.com/wp-json".to_string(),
        };

        let entity_id1 = repo
            .upsert_self_hosted_site(&mut test_conn, &site1)
            .expect("Failed to upsert site1");
        let entity_id2 = repo
            .upsert_self_hosted_site(&mut test_conn, &site2)
            .expect("Failed to upsert site2");

        // Verify different sites get different EntityIds
        assert_ne!(entity_id1, entity_id2);

        // Verify both can be retrieved by URL
        let retrieved1 = repo
            .select_self_hosted_site_by_url(&test_conn, &site1.url)
            .expect("Failed to select site by URL");
        let retrieved2 = repo
            .select_self_hosted_site_by_url(&test_conn, &site2.url)
            .expect("Failed to select site by URL");

        assert!(retrieved1.is_some());
        assert!(retrieved2.is_some());
    }

    #[rstest]
    fn test_delete_site_removes_both_tables(mut test_conn: Connection) {
        let repo = SiteRepository;
        let site = SelfHostedSite {
            url: "https://example.com".to_string(),
            api_root: "https://example.com/wp-json".to_string(),
        };

        // Create site
        let entity_id = repo
            .upsert_self_hosted_site(&mut test_conn, &site)
            .expect("Failed to upsert site");

        // Verify site exists in both tables
        let count_sites = repo
            .count_all_db_sites(&test_conn)
            .expect("Failed to count db_sites");
        let count_self_hosted = repo
            .count_all_self_hosted_sites(&test_conn)
            .expect("Failed to count self_hosted_sites");
        assert_eq!(count_sites, 1);
        assert_eq!(count_self_hosted, 1);

        // Delete site using DbSite from EntityId
        let deleted = repo
            .delete_site(&mut test_conn, &entity_id.db_site)
            .expect("Failed to delete site");
        assert!(deleted, "Should return true when site is deleted");

        // Verify site is removed from both tables
        let count_sites_after = repo
            .count_all_db_sites(&test_conn)
            .expect("Failed to count db_sites after delete");
        let count_self_hosted_after = repo
            .count_all_self_hosted_sites(&test_conn)
            .expect("Failed to count self_hosted_sites after delete");
        assert_eq!(
            count_sites_after, 0,
            "Site should be deleted from sites table"
        );
        assert_eq!(
            count_self_hosted_after, 0,
            "Site should be deleted from self_hosted_sites table"
        );
    }

    #[rstest]
    fn test_delete_site_returns_false_for_non_existent_site(mut test_conn: Connection) {
        let repo = SiteRepository;

        let non_existent_site = DbSite {
            row_id: RowId(999),
            site_type: DbSiteType::SelfHosted,
            mapped_site_id: RowId(888),
        };

        let deleted = repo
            .delete_site(&mut test_conn, &non_existent_site)
            .expect("Failed to delete site");
        assert!(!deleted, "Should return false when site doesn't exist");
    }

    #[rstest]
    fn test_delete_site_only_deletes_specified_site(mut test_conn: Connection) {
        let repo = SiteRepository;

        // Create two sites
        let site1 = SelfHostedSite {
            url: "https://example1.com".to_string(),
            api_root: "https://example1.com/wp-json".to_string(),
        };
        let site2 = SelfHostedSite {
            url: "https://example2.com".to_string(),
            api_root: "https://example2.com/wp-json".to_string(),
        };

        let entity_id1 = repo
            .upsert_self_hosted_site(&mut test_conn, &site1)
            .expect("Failed to upsert site1");
        let entity_id2 = repo
            .upsert_self_hosted_site(&mut test_conn, &site2)
            .expect("Failed to upsert site2");

        // Delete site1
        let deleted = repo
            .delete_site(&mut test_conn, &entity_id1.db_site)
            .expect("Failed to delete site1");
        assert!(deleted);

        // Verify site1 is gone
        let retrieved1 = repo
            .select_self_hosted_site_by_url(&test_conn, &site1.url)
            .expect("Failed to select site by URL");
        assert_eq!(retrieved1, None, "Site1 should be deleted");

        // Verify site2 still exists
        let retrieved2 = repo
            .select_self_hosted_site_by_url(&test_conn, &site2.url)
            .expect("Failed to select site by URL");
        assert!(retrieved2.is_some(), "Site2 should still exist");
        assert_eq!(
            retrieved2.expect("Site2 should exist").entity_id.as_ref(),
            &entity_id2
        );
    }

    #[rstest]
    fn test_delete_self_hosted_site_by_url_deletes_site(mut test_conn: Connection) {
        let repo = SiteRepository;
        let url = "https://example.com";
        let site = SelfHostedSite {
            url: url.to_string(),
            api_root: "https://example.com/wp-json".to_string(),
        };

        // Create site
        repo.upsert_self_hosted_site(&mut test_conn, &site)
            .expect("Failed to upsert site");

        // Verify site exists
        let before_delete = repo
            .select_self_hosted_site_by_url(&test_conn, url)
            .expect("Failed to select site by URL");
        assert!(before_delete.is_some());

        // Delete by URL
        let deleted = repo
            .delete_self_hosted_site_by_url(&mut test_conn, url)
            .expect("Failed to delete site by URL");
        assert!(deleted, "Should return true when site is deleted");

        // Verify site is gone
        let after_delete = repo
            .select_self_hosted_site_by_url(&test_conn, url)
            .expect("Failed to select site by URL");
        assert_eq!(after_delete, None, "Site should be deleted");
    }

    #[rstest]
    fn test_delete_self_hosted_site_by_url_returns_false_for_non_existent(
        mut test_conn: Connection,
    ) {
        let repo = SiteRepository;

        let deleted = repo
            .delete_self_hosted_site_by_url(&mut test_conn, "https://non-existent.com")
            .expect("Failed to delete site by URL");
        assert!(!deleted, "Should return false when site doesn't exist");
    }
}
