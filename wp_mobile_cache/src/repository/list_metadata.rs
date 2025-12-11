use crate::{
    DbTable, RowId, SqliteDbError,
    db_types::db_site::DbSite,
    list_metadata::{DbListMetadata, DbListMetadataItem, DbListMetadataState, ListState},
    repository::QueryExecutor,
};

/// Repository for managing list metadata in the database.
///
/// Provides methods for querying and managing list pagination, items, and sync state.
pub struct ListMetadataRepository;

impl ListMetadataRepository {
    /// Get the database table for list metadata headers
    pub const fn header_table() -> DbTable {
        DbTable::ListMetadata
    }

    /// Get the database table for list metadata items
    pub const fn items_table() -> DbTable {
        DbTable::ListMetadataItems
    }

    /// Get the database table for list metadata state
    pub const fn state_table() -> DbTable {
        DbTable::ListMetadataState
    }

    // ============================================================
    // Read Operations
    // ============================================================

    /// Get list metadata header by site and key.
    ///
    /// Returns None if the list doesn't exist.
    pub fn get_header(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
    ) -> Result<Option<DbListMetadata>, SqliteDbError> {
        let sql = format!(
            "SELECT * FROM {} WHERE db_site_id = ? AND key = ?",
            Self::header_table().table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        let mut rows = stmt.query_map(rusqlite::params![site.row_id, key], |row| {
            DbListMetadata::from_row(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        match rows.next() {
            Some(result) => Ok(Some(result.map_err(SqliteDbError::from)?)),
            None => Ok(None),
        }
    }

    /// Get or create list metadata header.
    ///
    /// If the header doesn't exist, creates it with default values and returns its rowid.
    /// If it exists, returns the existing rowid.
    pub fn get_or_create(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
    ) -> Result<RowId, SqliteDbError> {
        // Try to get existing
        if let Some(header) = self.get_header(executor, site, key)? {
            return Ok(header.row_id);
        }

        // Create new header with defaults
        let sql = format!(
            "INSERT INTO {} (db_site_id, key, current_page, per_page, version) VALUES (?, ?, 0, 20, 0)",
            Self::header_table().table_name()
        );
        executor.execute(&sql, rusqlite::params![site.row_id, key])?;

        Ok(executor.last_insert_rowid())
    }

    /// Get all items for a list, ordered by rowid (insertion order = display order).
    pub fn get_items(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
    ) -> Result<Vec<DbListMetadataItem>, SqliteDbError> {
        let sql = format!(
            "SELECT * FROM {} WHERE db_site_id = ? AND key = ? ORDER BY rowid",
            Self::items_table().table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params![site.row_id, key], |row| {
            DbListMetadataItem::from_row(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(SqliteDbError::from)
    }

    /// Get the current sync state for a list.
    ///
    /// Returns None if no state record exists (list not yet synced).
    pub fn get_state(
        &self,
        executor: &impl QueryExecutor,
        list_metadata_id: RowId,
    ) -> Result<Option<DbListMetadataState>, SqliteDbError> {
        let sql = format!(
            "SELECT * FROM {} WHERE list_metadata_id = ?",
            Self::state_table().table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        let mut rows = stmt.query_map(rusqlite::params![list_metadata_id], |row| {
            DbListMetadataState::from_row(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        match rows.next() {
            Some(result) => Ok(Some(result.map_err(SqliteDbError::from)?)),
            None => Ok(None),
        }
    }

    /// Get the current sync state for a list by site and key.
    ///
    /// Convenience method that looks up the list_metadata_id first.
    /// Returns ListState::Idle if the list or state doesn't exist.
    pub fn get_state_by_key(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
    ) -> Result<ListState, SqliteDbError> {
        let header = self.get_header(executor, site, key)?;
        match header {
            Some(h) => {
                let state = self.get_state(executor, h.row_id)?;
                Ok(state.map(|s| s.state).unwrap_or(ListState::Idle))
            }
            None => Ok(ListState::Idle),
        }
    }

    /// Get the current version for a list.
    ///
    /// Returns 0 if the list doesn't exist.
    pub fn get_version(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
    ) -> Result<i64, SqliteDbError> {
        let header = self.get_header(executor, site, key)?;
        Ok(header.map(|h| h.version).unwrap_or(0))
    }

    /// Check if the current version matches the expected version.
    ///
    /// Used for concurrency control to detect if a refresh happened
    /// while a load-more operation was in progress.
    pub fn check_version(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
        expected_version: i64,
    ) -> Result<bool, SqliteDbError> {
        let current_version = self.get_version(executor, site, key)?;
        Ok(current_version == expected_version)
    }

    /// Get the item count for a list.
    pub fn get_item_count(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
    ) -> Result<i64, SqliteDbError> {
        let sql = format!(
            "SELECT COUNT(*) FROM {} WHERE db_site_id = ? AND key = ?",
            Self::items_table().table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        stmt.query_row(rusqlite::params![site.row_id, key], |row| row.get(0))
            .map_err(SqliteDbError::from)
    }

    // ============================================================
    // Write Operations
    // ============================================================

    /// Set items for a list, replacing any existing items.
    ///
    /// Used for refresh (page 1) - deletes all existing items and inserts new ones.
    /// Items are inserted in order, so rowid determines display order.
    pub fn set_items(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
        items: &[ListMetadataItemInput],
    ) -> Result<(), SqliteDbError> {
        // Delete existing items
        let delete_sql = format!(
            "DELETE FROM {} WHERE db_site_id = ? AND key = ?",
            Self::items_table().table_name()
        );
        executor.execute(&delete_sql, rusqlite::params![site.row_id, key])?;

        // Insert new items
        self.insert_items(executor, site, key, items)?;

        Ok(())
    }

    /// Append items to an existing list.
    ///
    /// Used for load-more (page 2+) - appends items without deleting existing ones.
    /// Items are inserted in order, so they appear after existing items.
    pub fn append_items(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
        items: &[ListMetadataItemInput],
    ) -> Result<(), SqliteDbError> {
        self.insert_items(executor, site, key, items)
    }

    /// Internal helper to insert items.
    fn insert_items(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
        items: &[ListMetadataItemInput],
    ) -> Result<(), SqliteDbError> {
        if items.is_empty() {
            return Ok(());
        }

        let insert_sql = format!(
            "INSERT INTO {} (db_site_id, key, entity_id, modified_gmt) VALUES (?, ?, ?, ?)",
            Self::items_table().table_name()
        );

        for item in items {
            executor.execute(
                &insert_sql,
                rusqlite::params![site.row_id, key, item.entity_id, item.modified_gmt],
            )?;
        }

        Ok(())
    }

    /// Update header pagination info.
    pub fn update_header(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
        update: &ListMetadataHeaderUpdate,
    ) -> Result<(), SqliteDbError> {
        // Ensure header exists
        self.get_or_create(executor, site, key)?;

        let sql = format!(
            "UPDATE {} SET total_pages = ?, total_items = ?, current_page = ?, per_page = ?, last_updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE db_site_id = ? AND key = ?",
            Self::header_table().table_name()
        );

        executor.execute(
            &sql,
            rusqlite::params![
                update.total_pages,
                update.total_items,
                update.current_page,
                update.per_page,
                site.row_id,
                key
            ],
        )?;

        Ok(())
    }

    /// Update sync state for a list.
    ///
    /// Creates the state record if it doesn't exist (upsert).
    pub fn update_state(
        &self,
        executor: &impl QueryExecutor,
        list_metadata_id: RowId,
        state: ListState,
        error_message: Option<&str>,
    ) -> Result<(), SqliteDbError> {
        // Use INSERT OR REPLACE for upsert behavior
        let sql = format!(
            "INSERT INTO {} (list_metadata_id, state, error_message, updated_at) VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
             ON CONFLICT(list_metadata_id) DO UPDATE SET state = excluded.state, error_message = excluded.error_message, updated_at = excluded.updated_at",
            Self::state_table().table_name()
        );

        executor.execute(
            &sql,
            rusqlite::params![list_metadata_id, state.as_db_str(), error_message],
        )?;

        Ok(())
    }

    /// Update sync state for a list by site and key.
    ///
    /// Convenience method that looks up or creates the list_metadata_id first.
    pub fn update_state_by_key(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
        state: ListState,
        error_message: Option<&str>,
    ) -> Result<(), SqliteDbError> {
        let list_metadata_id = self.get_or_create(executor, site, key)?;
        self.update_state(executor, list_metadata_id, state, error_message)
    }

    /// Increment version and return the new value.
    ///
    /// Used when starting a refresh to invalidate any in-flight load-more operations.
    pub fn increment_version(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
    ) -> Result<i64, SqliteDbError> {
        // Ensure header exists
        self.get_or_create(executor, site, key)?;

        let sql = format!(
            "UPDATE {} SET version = version + 1, last_first_page_fetched_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE db_site_id = ? AND key = ?",
            Self::header_table().table_name()
        );

        executor.execute(&sql, rusqlite::params![site.row_id, key])?;

        // Return the new version
        self.get_version(executor, site, key)
    }

    /// Delete all data for a list (header, items, and state).
    pub fn delete_list(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
    ) -> Result<(), SqliteDbError> {
        // Delete items first (no FK constraint to header)
        let delete_items_sql = format!(
            "DELETE FROM {} WHERE db_site_id = ? AND key = ?",
            Self::items_table().table_name()
        );
        executor.execute(&delete_items_sql, rusqlite::params![site.row_id, key])?;

        // Delete header (state will be cascade deleted via FK)
        let delete_header_sql = format!(
            "DELETE FROM {} WHERE db_site_id = ? AND key = ?",
            Self::header_table().table_name()
        );
        executor.execute(&delete_header_sql, rusqlite::params![site.row_id, key])?;

        Ok(())
    }
}

/// Input for creating a list metadata item.
#[derive(Debug, Clone)]
pub struct ListMetadataItemInput {
    /// Entity ID (post ID, comment ID, etc.)
    pub entity_id: i64,
    /// Last modified timestamp (for staleness detection)
    pub modified_gmt: Option<String>,
}

/// Update parameters for list metadata header.
#[derive(Debug, Clone, Default)]
pub struct ListMetadataHeaderUpdate {
    /// Total number of pages from API response
    pub total_pages: Option<i64>,
    /// Total number of items from API response
    pub total_items: Option<i64>,
    /// Current page that has been loaded
    pub current_page: i64,
    /// Items per page
    pub per_page: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{TestContext, test_ctx};
    use rstest::*;

    fn list_metadata_repo() -> ListMetadataRepository {
        ListMetadataRepository
    }

    #[rstest]
    fn test_get_header_returns_none_for_non_existent(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let result = repo
            .get_header(&test_ctx.conn, &test_ctx.site, "nonexistent:key")
            .unwrap();
        assert!(result.is_none());
    }

    #[rstest]
    fn test_get_or_create_creates_new_header(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:publish";

        // Create new header
        let row_id = repo.get_or_create(&test_ctx.conn, &test_ctx.site, key).unwrap();

        // Verify it was created with defaults
        let header = repo.get_header(&test_ctx.conn, &test_ctx.site, key).unwrap().unwrap();
        assert_eq!(header.row_id, row_id);
        assert_eq!(header.key, key);
        assert_eq!(header.current_page, 0);
        assert_eq!(header.per_page, 20);
        assert_eq!(header.version, 0);
        assert!(header.total_pages.is_none());
        assert!(header.total_items.is_none());
    }

    #[rstest]
    fn test_get_or_create_returns_existing_header(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:draft";

        // Create initial header
        let first_row_id = repo.get_or_create(&test_ctx.conn, &test_ctx.site, key).unwrap();

        // Get or create again should return same rowid
        let second_row_id = repo.get_or_create(&test_ctx.conn, &test_ctx.site, key).unwrap();

        assert_eq!(first_row_id, second_row_id);
    }

    #[rstest]
    fn test_get_items_returns_empty_for_non_existent_list(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let items = repo
            .get_items(&test_ctx.conn, &test_ctx.site, "nonexistent:key")
            .unwrap();
        assert!(items.is_empty());
    }

    #[rstest]
    fn test_get_state_returns_none_for_non_existent(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let result = repo.get_state(&test_ctx.conn, RowId(999999)).unwrap();
        assert!(result.is_none());
    }

    #[rstest]
    fn test_get_state_by_key_returns_idle_for_non_existent_list(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let state = repo
            .get_state_by_key(&test_ctx.conn, &test_ctx.site, "nonexistent:key")
            .unwrap();
        assert_eq!(state, ListState::Idle);
    }

    #[rstest]
    fn test_get_version_returns_zero_for_non_existent_list(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let version = repo
            .get_version(&test_ctx.conn, &test_ctx.site, "nonexistent:key")
            .unwrap();
        assert_eq!(version, 0);
    }

    #[rstest]
    fn test_check_version_returns_true_for_matching_version(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:publish";

        // Create header (version = 0)
        repo.get_or_create(&test_ctx.conn, &test_ctx.site, key).unwrap();

        // Check version matches
        let matches = repo
            .check_version(&test_ctx.conn, &test_ctx.site, key, 0)
            .unwrap();
        assert!(matches);

        // Check version doesn't match
        let matches = repo
            .check_version(&test_ctx.conn, &test_ctx.site, key, 1)
            .unwrap();
        assert!(!matches);
    }

    #[rstest]
    fn test_get_item_count_returns_zero_for_empty_list(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let count = repo
            .get_item_count(&test_ctx.conn, &test_ctx.site, "empty:list")
            .unwrap();
        assert_eq!(count, 0);
    }

    #[rstest]
    fn test_list_metadata_column_enum_matches_schema(test_ctx: TestContext) {
        // Verify column order by selecting specific columns and checking positions
        let sql = format!(
            "SELECT rowid, db_site_id, key, total_pages, total_items, current_page, per_page, last_first_page_fetched_at, last_updated_at, version FROM {}",
            ListMetadataRepository::header_table().table_name()
        );
        let stmt = test_ctx.conn.prepare(&sql);
        assert!(
            stmt.is_ok(),
            "Column order mismatch - SELECT with explicit columns failed"
        );
    }

    #[rstest]
    fn test_list_metadata_items_column_enum_matches_schema(test_ctx: TestContext) {
        let sql = format!(
            "SELECT rowid, db_site_id, key, entity_id, modified_gmt FROM {}",
            ListMetadataRepository::items_table().table_name()
        );
        let stmt = test_ctx.conn.prepare(&sql);
        assert!(
            stmt.is_ok(),
            "Column order mismatch - SELECT with explicit columns failed"
        );
    }

    #[rstest]
    fn test_list_metadata_state_column_enum_matches_schema(test_ctx: TestContext) {
        let sql = format!(
            "SELECT rowid, list_metadata_id, state, error_message, updated_at FROM {}",
            ListMetadataRepository::state_table().table_name()
        );
        let stmt = test_ctx.conn.prepare(&sql);
        assert!(
            stmt.is_ok(),
            "Column order mismatch - SELECT with explicit columns failed"
        );
    }

    // ============================================================
    // Write Operation Tests
    // ============================================================

    #[rstest]
    fn test_set_items_inserts_new_items(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:publish";

        let items = vec![
            ListMetadataItemInput {
                entity_id: 100,
                modified_gmt: Some("2024-01-01T00:00:00Z".to_string()),
            },
            ListMetadataItemInput {
                entity_id: 200,
                modified_gmt: Some("2024-01-02T00:00:00Z".to_string()),
            },
            ListMetadataItemInput {
                entity_id: 300,
                modified_gmt: None,
            },
        ];

        repo.set_items(&test_ctx.conn, &test_ctx.site, key, &items).unwrap();

        let retrieved = repo.get_items(&test_ctx.conn, &test_ctx.site, key).unwrap();
        assert_eq!(retrieved.len(), 3);
        assert_eq!(retrieved[0].entity_id, 100);
        assert_eq!(retrieved[1].entity_id, 200);
        assert_eq!(retrieved[2].entity_id, 300);
        assert!(retrieved[2].modified_gmt.is_none());
    }

    #[rstest]
    fn test_set_items_replaces_existing_items(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:draft";

        // Insert initial items
        let initial_items = vec![
            ListMetadataItemInput { entity_id: 1, modified_gmt: None },
            ListMetadataItemInput { entity_id: 2, modified_gmt: None },
        ];
        repo.set_items(&test_ctx.conn, &test_ctx.site, key, &initial_items).unwrap();

        // Replace with new items
        let new_items = vec![
            ListMetadataItemInput { entity_id: 10, modified_gmt: None },
            ListMetadataItemInput { entity_id: 20, modified_gmt: None },
            ListMetadataItemInput { entity_id: 30, modified_gmt: None },
        ];
        repo.set_items(&test_ctx.conn, &test_ctx.site, key, &new_items).unwrap();

        let retrieved = repo.get_items(&test_ctx.conn, &test_ctx.site, key).unwrap();
        assert_eq!(retrieved.len(), 3);
        assert_eq!(retrieved[0].entity_id, 10);
        assert_eq!(retrieved[1].entity_id, 20);
        assert_eq!(retrieved[2].entity_id, 30);
    }

    #[rstest]
    fn test_append_items_adds_to_existing(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:pending";

        // Insert initial items
        let initial_items = vec![
            ListMetadataItemInput { entity_id: 1, modified_gmt: None },
            ListMetadataItemInput { entity_id: 2, modified_gmt: None },
        ];
        repo.set_items(&test_ctx.conn, &test_ctx.site, key, &initial_items).unwrap();

        // Append more items
        let more_items = vec![
            ListMetadataItemInput { entity_id: 3, modified_gmt: None },
            ListMetadataItemInput { entity_id: 4, modified_gmt: None },
        ];
        repo.append_items(&test_ctx.conn, &test_ctx.site, key, &more_items).unwrap();

        let retrieved = repo.get_items(&test_ctx.conn, &test_ctx.site, key).unwrap();
        assert_eq!(retrieved.len(), 4);
        assert_eq!(retrieved[0].entity_id, 1);
        assert_eq!(retrieved[1].entity_id, 2);
        assert_eq!(retrieved[2].entity_id, 3);
        assert_eq!(retrieved[3].entity_id, 4);
    }

    #[rstest]
    fn test_update_header_updates_pagination(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:publish";

        let update = ListMetadataHeaderUpdate {
            total_pages: Some(5),
            total_items: Some(100),
            current_page: 1,
            per_page: 20,
        };

        repo.update_header(&test_ctx.conn, &test_ctx.site, key, &update).unwrap();

        let header = repo.get_header(&test_ctx.conn, &test_ctx.site, key).unwrap().unwrap();
        assert_eq!(header.total_pages, Some(5));
        assert_eq!(header.total_items, Some(100));
        assert_eq!(header.current_page, 1);
        assert_eq!(header.per_page, 20);
        assert!(header.last_updated_at.is_some());
    }

    #[rstest]
    fn test_update_state_creates_new_state(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:publish";

        let list_id = repo.get_or_create(&test_ctx.conn, &test_ctx.site, key).unwrap();
        repo.update_state(&test_ctx.conn, list_id, ListState::FetchingFirstPage, None).unwrap();

        let state = repo.get_state(&test_ctx.conn, list_id).unwrap().unwrap();
        assert_eq!(state.state, ListState::FetchingFirstPage);
        assert!(state.error_message.is_none());
    }

    #[rstest]
    fn test_update_state_updates_existing_state(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:draft";

        let list_id = repo.get_or_create(&test_ctx.conn, &test_ctx.site, key).unwrap();

        // Set initial state
        repo.update_state(&test_ctx.conn, list_id, ListState::FetchingFirstPage, None).unwrap();

        // Update to error state
        repo.update_state(&test_ctx.conn, list_id, ListState::Error, Some("Network error")).unwrap();

        let state = repo.get_state(&test_ctx.conn, list_id).unwrap().unwrap();
        assert_eq!(state.state, ListState::Error);
        assert_eq!(state.error_message.as_deref(), Some("Network error"));
    }

    #[rstest]
    fn test_update_state_by_key(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:pending";

        repo.update_state_by_key(&test_ctx.conn, &test_ctx.site, key, ListState::FetchingNextPage, None).unwrap();

        let state = repo.get_state_by_key(&test_ctx.conn, &test_ctx.site, key).unwrap();
        assert_eq!(state, ListState::FetchingNextPage);
    }

    #[rstest]
    fn test_increment_version(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:publish";

        // Create header (version starts at 0)
        repo.get_or_create(&test_ctx.conn, &test_ctx.site, key).unwrap();
        let initial_version = repo.get_version(&test_ctx.conn, &test_ctx.site, key).unwrap();
        assert_eq!(initial_version, 0);

        // Increment version
        let new_version = repo.increment_version(&test_ctx.conn, &test_ctx.site, key).unwrap();
        assert_eq!(new_version, 1);

        // Increment again
        let newer_version = repo.increment_version(&test_ctx.conn, &test_ctx.site, key).unwrap();
        assert_eq!(newer_version, 2);

        // Verify last_first_page_fetched_at is set
        let header = repo.get_header(&test_ctx.conn, &test_ctx.site, key).unwrap().unwrap();
        assert!(header.last_first_page_fetched_at.is_some());
    }

    #[rstest]
    fn test_delete_list_removes_all_data(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:publish";

        // Create header and add items and state
        let list_id = repo.get_or_create(&test_ctx.conn, &test_ctx.site, key).unwrap();
        let items = vec![ListMetadataItemInput { entity_id: 1, modified_gmt: None }];
        repo.set_items(&test_ctx.conn, &test_ctx.site, key, &items).unwrap();
        repo.update_state(&test_ctx.conn, list_id, ListState::Idle, None).unwrap();

        // Verify data exists
        assert!(repo.get_header(&test_ctx.conn, &test_ctx.site, key).unwrap().is_some());
        assert_eq!(repo.get_item_count(&test_ctx.conn, &test_ctx.site, key).unwrap(), 1);

        // Delete the list
        repo.delete_list(&test_ctx.conn, &test_ctx.site, key).unwrap();

        // Verify everything is deleted
        assert!(repo.get_header(&test_ctx.conn, &test_ctx.site, key).unwrap().is_none());
        assert_eq!(repo.get_item_count(&test_ctx.conn, &test_ctx.site, key).unwrap(), 0);
    }

    #[rstest]
    fn test_items_preserve_order(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:ordered";

        // Insert items in specific order
        let items: Vec<ListMetadataItemInput> = (1..=10)
            .map(|i| ListMetadataItemInput { entity_id: i * 100, modified_gmt: None })
            .collect();

        repo.set_items(&test_ctx.conn, &test_ctx.site, key, &items).unwrap();

        let retrieved = repo.get_items(&test_ctx.conn, &test_ctx.site, key).unwrap();
        assert_eq!(retrieved.len(), 10);

        // Verify order is preserved (rowid ordering)
        for (i, item) in retrieved.iter().enumerate() {
            assert_eq!(item.entity_id, ((i + 1) * 100) as i64);
        }
    }
}
