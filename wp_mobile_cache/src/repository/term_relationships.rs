use crate::{DbSite, RowId, SqliteDbError, repository::QueryExecutor};
use std::collections::HashMap;
use wp_api::taxonomies::TaxonomyType;
use wp_api::terms::TermId;

/// Terms associated with a post (categories and tags).
///
/// This struct is used to populate term fields when constructing database entities,
/// ensuring that terms are always loaded from the term_relationships table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PostTerms {
    pub categories: Option<Vec<TermId>>,
    pub tags: Option<Vec<TermId>>,
}

/// Repository for managing term relationships in the database.
///
/// Provides methods for syncing, querying, and deleting term associations
/// between objects (posts, pages, etc.) and WordPress terms.
pub struct TermRelationshipRepository;

impl TermRelationshipRepository {
    const TABLE_NAME: &'static str = "term_relationships";

    /// Synchronize terms for an object (only insert new, delete removed, keep unchanged).
    ///
    /// This approach is observer-friendly: unchanged terms generate no DB events.
    /// Only actual changes (new terms added, old terms removed) generate INSERT/DELETE events.
    ///
    /// **IMPORTANT**: This method must be called within a transaction to ensure atomicity.
    /// The transaction parameter enforces this requirement at compile-time.
    pub fn sync_terms_for_object(
        &self,
        transaction: &rusqlite::Transaction<'_>,
        site: &DbSite,
        object_id: RowId,
        taxonomy_type: &TaxonomyType,
        new_term_ids: &[TermId],
    ) -> Result<(), SqliteDbError> {
        // 1. Get existing term IDs
        let existing_terms =
            self.get_terms_for_object(transaction, site, object_id, taxonomy_type)?;

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
            self.delete_terms(transaction, site, object_id, taxonomy_type, &to_delete)?;
        }

        // 4. Insert new terms (only the ones being added)
        if !to_insert.is_empty() {
            self.insert_terms(transaction, site, object_id, taxonomy_type, &to_insert)?;
        }

        // Unchanged terms: no DB operations = no observer events
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
            "DELETE FROM {} WHERE db_site_id = ? AND object_id = ? AND taxonomy_type = ? AND term_id IN ({})",
            Self::TABLE_NAME,
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

        let insert_sql = format!(
            "INSERT INTO {} (db_site_id, object_id, term_id, taxonomy_type) VALUES (?, ?, ?, ?)",
            Self::TABLE_NAME
        );

        for term_id in term_ids {
            executor.execute(
                &insert_sql,
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
        let sql = format!(
            "SELECT term_id FROM {} WHERE db_site_id = ? AND object_id = ? AND taxonomy_type = ?",
            Self::TABLE_NAME
        );
        let mut stmt = executor.prepare(&sql)?;
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
        let sql = format!(
            "SELECT taxonomy_type, term_id FROM {} WHERE db_site_id = ? AND object_id = ?",
            Self::TABLE_NAME
        );
        let mut stmt = executor.prepare(&sql)?;
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
        let sql = format!(
            "DELETE FROM {} WHERE db_site_id = ? AND object_id = ?",
            Self::TABLE_NAME
        );
        executor.execute(&sql, rusqlite::params![site.row_id, object_id])
    }

    /// Get post terms (categories and tags) for a single post object.
    ///
    /// This is the canonical way to retrieve terms for constructing post entities.
    /// Returns `None` for both categories and tags if no terms exist for the object.
    pub fn get_post_terms(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        object_id: RowId,
    ) -> Result<PostTerms, SqliteDbError> {
        let terms_map = self.get_all_terms_for_object(executor, site, object_id)?;
        Ok(PostTerms {
            categories: terms_map.get(&TaxonomyType::Category).cloned(),
            tags: terms_map.get(&TaxonomyType::PostTag).cloned(),
        })
    }

    /// Get post terms for multiple objects in a single batch query.
    ///
    /// This is more efficient than calling `get_post_terms` in a loop when loading
    /// multiple posts. Returns a HashMap mapping object_id to its PostTerms.
    /// Objects without any terms will have an empty PostTerms (both fields None).
    pub fn get_post_terms_batch(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        object_ids: &[RowId],
    ) -> Result<HashMap<RowId, PostTerms>, SqliteDbError> {
        if object_ids.is_empty() {
            return Ok(HashMap::new());
        }

        // Build placeholders for IN clause
        let placeholders: Vec<_> = (0..object_ids.len()).map(|_| "?").collect();
        let sql = format!(
            "SELECT object_id, taxonomy_type, term_id FROM {} WHERE db_site_id = ? AND object_id IN ({})",
            Self::TABLE_NAME,
            placeholders.join(", ")
        );

        // Build params: [site_id, object_id1, object_id2, ...]
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(site.row_id)];
        for object_id in object_ids {
            params.push(Box::new(*object_id));
        }

        let params_refs: Vec<_> = params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = executor.prepare(&sql)?;
        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            let object_id: RowId = row.get(0)?;
            let taxonomy_str: String = row.get(1)?;
            let term_id: i64 = row.get(2)?;
            Ok((object_id, taxonomy_str, term_id))
        })?;

        // Group terms by object_id and taxonomy_type
        let mut object_terms: HashMap<RowId, HashMap<TaxonomyType, Vec<TermId>>> = HashMap::new();
        for row_result in rows {
            let (object_id, taxonomy_str, term_id) = row_result.map_err(SqliteDbError::from)?;
            let taxonomy_type: TaxonomyType = serde_json::from_value(serde_json::Value::String(
                taxonomy_str.clone(),
            ))
            .map_err(|e| {
                SqliteDbError::SqliteError(format!(
                    "Invalid taxonomy_type '{}': {}",
                    taxonomy_str, e
                ))
            })?;

            object_terms
                .entry(object_id)
                .or_default()
                .entry(taxonomy_type)
                .or_default()
                .push(TermId(term_id));
        }

        // Convert to PostTerms, ensuring all requested object_ids have an entry
        let mut result = HashMap::new();
        for &object_id in object_ids {
            let terms_map = object_terms.get(&object_id);
            result.insert(
                object_id,
                PostTerms {
                    categories: terms_map.and_then(|m| m.get(&TaxonomyType::Category).cloned()),
                    tags: terms_map.and_then(|m| m.get(&TaxonomyType::PostTag).cloned()),
                },
            );
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{TestContext, test_ctx};
    use rstest::*;

    #[rstest]
    fn test_sync_terms_insert_new(mut test_ctx: TestContext) {
        let test_object_id = RowId(42);

        let term_ids = vec![TermId(1), TermId(2), TermId(3)];

        // Sync terms (should insert all)
        let tx = test_ctx.conn.transaction().unwrap();
        test_ctx
            .term_repo
            .sync_terms_for_object(
                &tx,
                &test_ctx.site,
                test_object_id,
                &TaxonomyType::Category,
                &term_ids,
            )
            .unwrap();
        tx.commit().unwrap();

        // Verify all were inserted
        let retrieved = test_ctx
            .term_repo
            .get_terms_for_object(
                &test_ctx.conn,
                &test_ctx.site,
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
    fn test_sync_terms_remove_old(mut test_ctx: TestContext) {
        let test_object_id = RowId(42);

        // Insert initial terms
        let initial_terms = vec![TermId(1), TermId(2), TermId(3)];
        let tx = test_ctx.conn.transaction().unwrap();
        test_ctx
            .term_repo
            .sync_terms_for_object(
                &tx,
                &test_ctx.site,
                test_object_id,
                &TaxonomyType::PostTag,
                &initial_terms,
            )
            .unwrap();
        tx.commit().unwrap();

        // Sync with fewer terms (remove 2 and 3)
        let updated_terms = vec![TermId(1)];
        let tx = test_ctx.conn.transaction().unwrap();
        test_ctx
            .term_repo
            .sync_terms_for_object(
                &tx,
                &test_ctx.site,
                test_object_id,
                &TaxonomyType::PostTag,
                &updated_terms,
            )
            .unwrap();
        tx.commit().unwrap();

        // Verify only term 1 remains
        let retrieved = test_ctx
            .term_repo
            .get_terms_for_object(
                &test_ctx.conn,
                &test_ctx.site,
                test_object_id,
                &TaxonomyType::PostTag,
            )
            .unwrap();

        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0], TermId(1));
    }

    #[rstest]
    fn test_sync_terms_add_new_keep_existing(mut test_ctx: TestContext) {
        let test_object_id = RowId(42);

        // Insert initial terms
        let initial_terms = vec![TermId(1), TermId(2)];
        let tx = test_ctx.conn.transaction().unwrap();
        test_ctx
            .term_repo
            .sync_terms_for_object(
                &tx,
                &test_ctx.site,
                test_object_id,
                &TaxonomyType::Category,
                &initial_terms,
            )
            .unwrap();
        tx.commit().unwrap();

        // Sync with additional terms (keep 1, 2, add 3, 4)
        let updated_terms = vec![TermId(1), TermId(2), TermId(3), TermId(4)];
        let tx = test_ctx.conn.transaction().unwrap();
        test_ctx
            .term_repo
            .sync_terms_for_object(
                &tx,
                &test_ctx.site,
                test_object_id,
                &TaxonomyType::Category,
                &updated_terms,
            )
            .unwrap();
        tx.commit().unwrap();

        // Verify all four are present
        let retrieved = test_ctx
            .term_repo
            .get_terms_for_object(
                &test_ctx.conn,
                &test_ctx.site,
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
    fn test_sync_terms_no_changes(mut test_ctx: TestContext) {
        let test_object_id = RowId(42);

        // Insert initial terms
        let terms = vec![TermId(1), TermId(2), TermId(3)];
        let tx = test_ctx.conn.transaction().unwrap();
        test_ctx
            .term_repo
            .sync_terms_for_object(
                &tx,
                &test_ctx.site,
                test_object_id,
                &TaxonomyType::PostTag,
                &terms,
            )
            .unwrap();
        tx.commit().unwrap();

        // Sync with same terms (no changes)
        let tx = test_ctx.conn.transaction().unwrap();
        test_ctx
            .term_repo
            .sync_terms_for_object(
                &tx,
                &test_ctx.site,
                test_object_id,
                &TaxonomyType::PostTag,
                &terms,
            )
            .unwrap();
        tx.commit().unwrap();

        // Verify terms unchanged
        let retrieved = test_ctx
            .term_repo
            .get_terms_for_object(
                &test_ctx.conn,
                &test_ctx.site,
                test_object_id,
                &TaxonomyType::PostTag,
            )
            .unwrap();

        assert_eq!(retrieved.len(), 3);
    }

    #[rstest]
    fn test_get_all_terms_for_object(mut test_ctx: TestContext) {
        let test_object_id = RowId(42);

        // Add categories
        let categories = vec![TermId(1), TermId(2)];
        let tx = test_ctx.conn.transaction().unwrap();
        test_ctx
            .term_repo
            .sync_terms_for_object(
                &tx,
                &test_ctx.site,
                test_object_id,
                &TaxonomyType::Category,
                &categories,
            )
            .unwrap();
        tx.commit().unwrap();

        // Add tags
        let tags = vec![TermId(10), TermId(20), TermId(30)];
        let tx = test_ctx.conn.transaction().unwrap();
        test_ctx
            .term_repo
            .sync_terms_for_object(
                &tx,
                &test_ctx.site,
                test_object_id,
                &TaxonomyType::PostTag,
                &tags,
            )
            .unwrap();
        tx.commit().unwrap();

        // Get all terms
        let all_terms = test_ctx
            .term_repo
            .get_all_terms_for_object(&test_ctx.conn, &test_ctx.site, test_object_id)
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
    fn test_delete_all_terms_for_object(mut test_ctx: TestContext) {
        let test_object_id = RowId(42);

        // Add terms
        let tx = test_ctx.conn.transaction().unwrap();
        test_ctx
            .term_repo
            .sync_terms_for_object(
                &tx,
                &test_ctx.site,
                test_object_id,
                &TaxonomyType::Category,
                &[TermId(1)],
            )
            .unwrap();
        test_ctx
            .term_repo
            .sync_terms_for_object(
                &tx,
                &test_ctx.site,
                test_object_id,
                &TaxonomyType::PostTag,
                &[TermId(10)],
            )
            .unwrap();
        tx.commit().unwrap();

        // Delete all terms
        let deleted = test_ctx
            .term_repo
            .delete_all_terms_for_object(&test_ctx.conn, &test_ctx.site, test_object_id)
            .unwrap();
        assert_eq!(deleted, 2);

        // Verify all deleted
        let all_terms = test_ctx
            .term_repo
            .get_all_terms_for_object(&test_ctx.conn, &test_ctx.site, test_object_id)
            .unwrap();
        assert!(all_terms.is_empty());
    }

    #[rstest]
    fn test_different_taxonomy_types_are_isolated(mut test_ctx: TestContext) {
        let test_object_id = RowId(42);

        // Add same term ID to different taxonomies
        let tx = test_ctx.conn.transaction().unwrap();
        test_ctx
            .term_repo
            .sync_terms_for_object(
                &tx,
                &test_ctx.site,
                test_object_id,
                &TaxonomyType::Category,
                &[TermId(1)],
            )
            .unwrap();
        test_ctx
            .term_repo
            .sync_terms_for_object(
                &tx,
                &test_ctx.site,
                test_object_id,
                &TaxonomyType::PostTag,
                &[TermId(1)],
            )
            .unwrap();
        tx.commit().unwrap();

        // Verify both exist independently
        let categories = test_ctx
            .term_repo
            .get_terms_for_object(
                &test_ctx.conn,
                &test_ctx.site,
                test_object_id,
                &TaxonomyType::Category,
            )
            .unwrap();
        let tags = test_ctx
            .term_repo
            .get_terms_for_object(
                &test_ctx.conn,
                &test_ctx.site,
                test_object_id,
                &TaxonomyType::PostTag,
            )
            .unwrap();

        assert_eq!(categories.len(), 1);
        assert_eq!(tags.len(), 1);
    }
}
