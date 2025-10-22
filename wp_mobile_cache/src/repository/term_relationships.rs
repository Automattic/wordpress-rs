use crate::{DbSite, RowId, SqliteDbError, repository::QueryExecutor};
use std::collections::HashMap;
use wp_api::taxonomies::TaxonomyType;
use wp_api::terms::TermId;

/// Repository for managing term relationships in the database.
///
/// Provides methods for syncing, querying, and deleting term associations
/// between objects (posts, pages, etc.) and WordPress terms.
pub struct TermRelationshipRepository;

impl TermRelationshipRepository {
    /// Synchronize terms for an object (only insert new, delete removed, keep unchanged).
    ///
    /// This approach is observer-friendly: unchanged terms generate no DB events.
    /// Only actual changes (new terms added, old terms removed) generate INSERT/DELETE events.
    pub fn sync_terms_for_object(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        object_id: RowId,
        taxonomy_type: &TaxonomyType,
        new_term_ids: &[TermId],
    ) -> Result<(), SqliteDbError> {
        // 1. Get existing term IDs
        let existing_terms = self.get_terms_for_object(executor, site, object_id, taxonomy_type)?;

        // 2. Calculate diff (using Vec-based filtering since TermId may not impl Hash)
        let to_delete: Vec<_> = existing_terms
            .iter()
            .filter(|existing| !new_term_ids.contains(existing))
            .copied()
            .collect();

        let to_insert: Vec<_> = new_term_ids
            .iter()
            .filter(|new_id| !existing_terms.contains(new_id))
            .copied()
            .collect();

        // 3. Delete removed terms (only the ones being removed)
        if !to_delete.is_empty() {
            self.delete_terms(executor, site, object_id, taxonomy_type, &to_delete)?;
        }

        // 4. Insert new terms (only the ones being added)
        if !to_insert.is_empty() {
            self.insert_terms(executor, site, object_id, taxonomy_type, &to_insert)?;
        }

        // Unchanged terms: no DB operations = no observer events ✅
        Ok(())
    }

    /// Delete specific terms for an object.
    fn delete_terms(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        object_id: RowId,
        taxonomy_type: &TaxonomyType,
        term_ids: &[TermId],
    ) -> Result<(), SqliteDbError> {
        if term_ids.is_empty() {
            return Ok(());
        }

        // Build placeholders for IN clause
        let placeholders: Vec<_> = (0..term_ids.len()).map(|_| "?").collect();
        let sql = format!(
            "DELETE FROM term_relationships WHERE db_site_id = ? AND object_id = ? AND taxonomy_type = ? AND term_id IN ({})",
            placeholders.join(", ")
        );

        // Build params: [site_id, object_id, taxonomy_type, term_id1, term_id2, ...]
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![
            Box::new(site.row_id),
            Box::new(object_id),
            Box::new(taxonomy_type.to_string()),
        ];
        for term_id in term_ids {
            params.push(Box::new(term_id.0));
        }

        let params_refs: Vec<_> = params.iter().map(|p| p.as_ref()).collect();
        executor.execute(&sql, params_refs.as_slice())?;
        Ok(())
    }

    /// Insert new terms for an object.
    fn insert_terms(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        object_id: RowId,
        taxonomy_type: &TaxonomyType,
        term_ids: &[TermId],
    ) -> Result<(), SqliteDbError> {
        if term_ids.is_empty() {
            return Ok(());
        }

        for term_id in term_ids {
            executor.execute(
                "INSERT INTO term_relationships (db_site_id, object_id, term_id, taxonomy_type) VALUES (?, ?, ?, ?)",
                rusqlite::params![site.row_id, object_id, term_id.0, taxonomy_type.to_string()],
            )?;
        }
        Ok(())
    }

    /// Get all term IDs for an object's taxonomy.
    pub fn get_terms_for_object(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        object_id: RowId,
        taxonomy_type: &TaxonomyType,
    ) -> Result<Vec<TermId>, SqliteDbError> {
        let sql = "SELECT term_id FROM term_relationships WHERE db_site_id = ? AND object_id = ? AND taxonomy_type = ?";
        let mut stmt = executor.prepare(sql)?;
        let rows = stmt.query_map(
            rusqlite::params![site.row_id, object_id, taxonomy_type.to_string()],
            |row| {
                let id: i64 = row.get(0)?;
                Ok(TermId(id))
            },
        )?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(SqliteDbError::from)
    }

    /// Get all term IDs grouped by taxonomy for an object (for post reads with joins).
    pub fn get_all_terms_for_object(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        object_id: RowId,
    ) -> Result<HashMap<TaxonomyType, Vec<TermId>>, SqliteDbError> {
        let sql = "SELECT taxonomy_type, term_id FROM term_relationships WHERE db_site_id = ? AND object_id = ?";
        let mut stmt = executor.prepare(sql)?;
        let rows = stmt.query_map(rusqlite::params![site.row_id, object_id], |row| {
            let taxonomy_str: String = row.get(0)?;
            let term_id: i64 = row.get(1)?;
            Ok((taxonomy_str, term_id))
        })?;

        let mut result: HashMap<TaxonomyType, Vec<TermId>> = HashMap::new();
        for row_result in rows {
            let (taxonomy_str, term_id) = row_result.map_err(SqliteDbError::from)?;
            let taxonomy_type: TaxonomyType = serde_json::from_value(serde_json::Value::String(
                taxonomy_str.clone(),
            ))
            .map_err(|e| {
                SqliteDbError::SqliteError(format!(
                    "Invalid taxonomy_type '{}': {}",
                    taxonomy_str, e
                ))
            })?;

            result
                .entry(taxonomy_type)
                .or_default()
                .push(TermId(term_id));
        }

        Ok(result)
    }

    /// Delete all terms for an object (called when deleting the object itself).
    pub fn delete_all_terms_for_object(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        object_id: RowId,
    ) -> Result<usize, SqliteDbError> {
        executor.execute(
            "DELETE FROM term_relationships WHERE db_site_id = ? AND object_id = ?",
            rusqlite::params![site.row_id, object_id],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{test_db, test_site};
    use rstest::*;
    use rusqlite::Connection;

    #[rstest]
    fn test_sync_terms_insert_new(test_db: Connection, test_site: DbSite) {
        let repo = TermRelationshipRepository;
        let test_object_id = RowId(42);

        let term_ids = vec![TermId(1), TermId(2), TermId(3)];

        // Sync terms (should insert all)
        repo.sync_terms_for_object(
            &test_db,
            &test_site,
            test_object_id,
            &TaxonomyType::Category,
            &term_ids,
        )
        .unwrap();

        // Verify all were inserted
        let retrieved = repo
            .get_terms_for_object(
                &test_db,
                &test_site,
                test_object_id,
                &TaxonomyType::Category,
            )
            .unwrap();

        assert_eq!(retrieved.len(), 3);
        assert!(retrieved.contains(&TermId(1)));
        assert!(retrieved.contains(&TermId(2)));
        assert!(retrieved.contains(&TermId(3)));
    }

    #[rstest]
    fn test_sync_terms_remove_old(test_db: Connection, test_site: DbSite) {
        let repo = TermRelationshipRepository;
        let test_object_id = RowId(42);

        // Insert initial terms
        let initial_terms = vec![TermId(1), TermId(2), TermId(3)];
        repo.sync_terms_for_object(
            &test_db,
            &test_site,
            test_object_id,
            &TaxonomyType::PostTag,
            &initial_terms,
        )
        .unwrap();

        // Sync with fewer terms (remove 2 and 3)
        let updated_terms = vec![TermId(1)];
        repo.sync_terms_for_object(
            &test_db,
            &test_site,
            test_object_id,
            &TaxonomyType::PostTag,
            &updated_terms,
        )
        .unwrap();

        // Verify only term 1 remains
        let retrieved = repo
            .get_terms_for_object(&test_db, &test_site, test_object_id, &TaxonomyType::PostTag)
            .unwrap();

        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0], TermId(1));
    }

    #[rstest]
    fn test_sync_terms_add_new_keep_existing(test_db: Connection, test_site: DbSite) {
        let repo = TermRelationshipRepository;
        let test_object_id = RowId(42);

        // Insert initial terms
        let initial_terms = vec![TermId(1), TermId(2)];
        repo.sync_terms_for_object(
            &test_db,
            &test_site,
            test_object_id,
            &TaxonomyType::Category,
            &initial_terms,
        )
        .unwrap();

        // Sync with additional terms (keep 1, 2, add 3, 4)
        let updated_terms = vec![TermId(1), TermId(2), TermId(3), TermId(4)];
        repo.sync_terms_for_object(
            &test_db,
            &test_site,
            test_object_id,
            &TaxonomyType::Category,
            &updated_terms,
        )
        .unwrap();

        // Verify all four are present
        let retrieved = repo
            .get_terms_for_object(
                &test_db,
                &test_site,
                test_object_id,
                &TaxonomyType::Category,
            )
            .unwrap();

        assert_eq!(retrieved.len(), 4);
        assert!(retrieved.contains(&TermId(1)));
        assert!(retrieved.contains(&TermId(2)));
        assert!(retrieved.contains(&TermId(3)));
        assert!(retrieved.contains(&TermId(4)));
    }

    #[rstest]
    fn test_sync_terms_no_changes(test_db: Connection, test_site: DbSite) {
        let repo = TermRelationshipRepository;
        let test_object_id = RowId(42);

        // Insert initial terms
        let terms = vec![TermId(1), TermId(2), TermId(3)];
        repo.sync_terms_for_object(
            &test_db,
            &test_site,
            test_object_id,
            &TaxonomyType::PostTag,
            &terms,
        )
        .unwrap();

        // Sync with same terms (no changes)
        repo.sync_terms_for_object(
            &test_db,
            &test_site,
            test_object_id,
            &TaxonomyType::PostTag,
            &terms,
        )
        .unwrap();

        // Verify terms unchanged
        let retrieved = repo
            .get_terms_for_object(&test_db, &test_site, test_object_id, &TaxonomyType::PostTag)
            .unwrap();

        assert_eq!(retrieved.len(), 3);
    }

    #[rstest]
    fn test_get_all_terms_for_object(test_db: Connection, test_site: DbSite) {
        let repo = TermRelationshipRepository;
        let test_object_id = RowId(42);

        // Add categories
        let categories = vec![TermId(1), TermId(2)];
        repo.sync_terms_for_object(
            &test_db,
            &test_site,
            test_object_id,
            &TaxonomyType::Category,
            &categories,
        )
        .unwrap();

        // Add tags
        let tags = vec![TermId(10), TermId(20), TermId(30)];
        repo.sync_terms_for_object(
            &test_db,
            &test_site,
            test_object_id,
            &TaxonomyType::PostTag,
            &tags,
        )
        .unwrap();

        // Get all terms
        let all_terms = repo
            .get_all_terms_for_object(&test_db, &test_site, test_object_id)
            .unwrap();

        // Verify categories
        assert_eq!(all_terms.get(&TaxonomyType::Category).unwrap().len(), 2);
        assert!(
            all_terms
                .get(&TaxonomyType::Category)
                .unwrap()
                .contains(&TermId(1))
        );
        assert!(
            all_terms
                .get(&TaxonomyType::Category)
                .unwrap()
                .contains(&TermId(2))
        );

        // Verify tags
        assert_eq!(all_terms.get(&TaxonomyType::PostTag).unwrap().len(), 3);
        assert!(
            all_terms
                .get(&TaxonomyType::PostTag)
                .unwrap()
                .contains(&TermId(10))
        );
        assert!(
            all_terms
                .get(&TaxonomyType::PostTag)
                .unwrap()
                .contains(&TermId(20))
        );
        assert!(
            all_terms
                .get(&TaxonomyType::PostTag)
                .unwrap()
                .contains(&TermId(30))
        );
    }

    #[rstest]
    fn test_delete_all_terms_for_object(test_db: Connection, test_site: DbSite) {
        let repo = TermRelationshipRepository;
        let test_object_id = RowId(42);

        // Add terms
        repo.sync_terms_for_object(
            &test_db,
            &test_site,
            test_object_id,
            &TaxonomyType::Category,
            &[TermId(1)],
        )
        .unwrap();
        repo.sync_terms_for_object(
            &test_db,
            &test_site,
            test_object_id,
            &TaxonomyType::PostTag,
            &[TermId(10)],
        )
        .unwrap();

        // Delete all terms
        let deleted = repo
            .delete_all_terms_for_object(&test_db, &test_site, test_object_id)
            .unwrap();
        assert_eq!(deleted, 2);

        // Verify all deleted
        let all_terms = repo
            .get_all_terms_for_object(&test_db, &test_site, test_object_id)
            .unwrap();
        assert!(all_terms.is_empty());
    }

    #[rstest]
    fn test_different_taxonomy_types_are_isolated(test_db: Connection, test_site: DbSite) {
        let repo = TermRelationshipRepository;
        let test_object_id = RowId(42);

        // Add same term ID to different taxonomies
        repo.sync_terms_for_object(
            &test_db,
            &test_site,
            test_object_id,
            &TaxonomyType::Category,
            &[TermId(1)],
        )
        .unwrap();
        repo.sync_terms_for_object(
            &test_db,
            &test_site,
            test_object_id,
            &TaxonomyType::PostTag,
            &[TermId(1)],
        )
        .unwrap();

        // Verify both exist independently
        let categories = repo
            .get_terms_for_object(
                &test_db,
                &test_site,
                test_object_id,
                &TaxonomyType::Category,
            )
            .unwrap();
        let tags = repo
            .get_terms_for_object(&test_db, &test_site, test_object_id, &TaxonomyType::PostTag)
            .unwrap();

        assert_eq!(categories.len(), 1);
        assert_eq!(tags.len(), 1);
    }
}
