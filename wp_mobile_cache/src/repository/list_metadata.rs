use crate::{
    DbTable, RowId, SqliteDbError,
    db_types::db_site::DbSite,
    list_metadata::{
        DbListHeaderWithState, DbListMetadata, DbListMetadataItem, DbListMetadataState, ListState,
    },
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

    /// Get header with state in a single JOIN query.
    ///
    /// Returns pagination info + sync state combined. More efficient than
    /// calling `get_header()` and `get_state()` separately when both are needed.
    ///
    /// Returns `None` if the list doesn't exist.
    pub fn get_header_with_state(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
    ) -> Result<Option<DbListHeaderWithState>, SqliteDbError> {
        let sql = format!(
            "SELECT m.total_pages, m.total_items, m.current_page, m.per_page, s.state, s.error_message \
             FROM {} m \
             LEFT JOIN {} s ON s.list_metadata_id = m.rowid \
             WHERE m.db_site_id = ? AND m.key = ?",
            Self::header_table().table_name(),
            Self::state_table().table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        let mut rows = stmt.query_map(rusqlite::params![site.row_id, key], |row| {
            DbListHeaderWithState::from_row(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        match rows.next() {
            Some(result) => Ok(Some(result.map_err(SqliteDbError::from)?)),
            None => Ok(None),
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
            "INSERT INTO {} (db_site_id, key, entity_id, modified_gmt, parent, menu_order) VALUES (?, ?, ?, ?, ?, ?)",
            Self::items_table().table_name()
        );

        for item in items {
            executor.execute(
                &insert_sql,
                rusqlite::params![
                    site.row_id,
                    key,
                    item.entity_id,
                    item.modified_gmt,
                    item.parent,
                    item.menu_order
                ],
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
            "UPDATE {} SET total_pages = ?, total_items = ?, current_page = ?, per_page = ?, last_fetched_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE db_site_id = ? AND key = ?",
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

    // ============================================================
    // Concurrency Helpers
    // ============================================================

    /// Begin a refresh operation (fetch first page).
    ///
    /// Atomically:
    /// 1. Creates header if needed
    /// 2. Increments version (invalidates any in-flight load-more)
    /// 3. Updates state to FetchingFirstPage
    /// 4. Returns info needed for the fetch
    ///
    /// Call this before starting an API fetch for page 1.
    pub fn begin_refresh(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
    ) -> Result<RefreshInfo, SqliteDbError> {
        // Ensure header exists and get its ID
        let list_metadata_id = self.get_or_create(executor, site, key)?;

        // Increment version (invalidates any in-flight load-more)
        let version = self.increment_version(executor, site, key)?;

        // Update state to fetching
        self.update_state(
            executor,
            list_metadata_id,
            ListState::FetchingFirstPage,
            None,
        )?;

        // Get header for pagination info
        let header = self.get_header(executor, site, key)?.unwrap();

        Ok(RefreshInfo {
            list_metadata_id,
            version,
            per_page: header.per_page,
        })
    }

    /// Begin a load-next-page operation.
    ///
    /// Atomically:
    /// 1. Gets current pagination state
    /// 2. Checks if there are more pages to load
    /// 3. Updates state to FetchingNextPage
    /// 4. Returns info needed for the fetch (including version for later check)
    ///
    /// Returns None if already at the last page or no pages loaded yet.
    /// Call this before starting an API fetch for page N+1.
    pub fn begin_fetch_next_page(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
    ) -> Result<Option<FetchNextPageInfo>, SqliteDbError> {
        let header = match self.get_header(executor, site, key)? {
            Some(h) => h,
            None => return Ok(None), // List doesn't exist
        };

        // Check if we have pages loaded and more to fetch
        if header.current_page == 0 {
            return Ok(None); // No pages loaded yet, need refresh first
        }

        if let Some(total_pages) = header.total_pages
            && header.current_page >= total_pages
        {
            return Ok(None); // Already at last page
        }

        let next_page = header.current_page + 1;

        // Update state to fetching
        self.update_state(executor, header.row_id, ListState::FetchingNextPage, None)?;

        Ok(Some(FetchNextPageInfo {
            list_metadata_id: header.row_id,
            page: next_page,
            version: header.version,
            per_page: header.per_page,
        }))
    }

    /// Complete a sync operation successfully.
    ///
    /// Updates state to Idle and clears any error message.
    pub fn complete_sync(
        &self,
        executor: &impl QueryExecutor,
        list_metadata_id: RowId,
    ) -> Result<(), SqliteDbError> {
        self.update_state(executor, list_metadata_id, ListState::Idle, None)
    }

    /// Complete a sync operation with an error.
    ///
    /// Updates state to Error with the provided message.
    pub fn complete_sync_with_error(
        &self,
        executor: &impl QueryExecutor,
        list_metadata_id: RowId,
        error_message: &str,
    ) -> Result<(), SqliteDbError> {
        self.update_state(
            executor,
            list_metadata_id,
            ListState::Error,
            Some(error_message),
        )
    }

    // ============================================
    // Relevance checking for update hooks
    // ============================================

    /// Get the list_metadata_id (rowid) for a given key.
    ///
    /// Returns None if no list exists for this key yet.
    /// Used by collections to cache the ID for relevance checking.
    pub fn get_list_metadata_id(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
    ) -> Result<Option<RowId>, SqliteDbError> {
        self.get_header(executor, site, key)
            .map(|opt| opt.map(|h| h.row_id))
    }

    /// Get the list_metadata_id that a state row belongs to.
    ///
    /// Given a rowid from the list_metadata_state table, returns the
    /// list_metadata_id (FK to list_metadata) that this state belongs to.
    /// Returns None if the state row doesn't exist.
    pub fn get_list_metadata_id_for_state_row(
        &self,
        executor: &impl QueryExecutor,
        state_row_id: RowId,
    ) -> Result<Option<RowId>, SqliteDbError> {
        let sql = format!(
            "SELECT list_metadata_id FROM {} WHERE rowid = ?",
            Self::state_table().table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        let result = stmt.query_row([state_row_id], |row| row.get::<_, RowId>(0));

        match result {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(SqliteDbError::from(e)),
        }
    }

    /// Check if a list_metadata_items row belongs to a specific key.
    ///
    /// Given a rowid from the list_metadata_items table, checks if the item
    /// belongs to the list identified by (site, key).
    pub fn is_item_row_for_key(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        key: &str,
        item_row_id: RowId,
    ) -> Result<bool, SqliteDbError> {
        let sql = format!(
            "SELECT 1 FROM {} WHERE rowid = ? AND db_site_id = ? AND key = ?",
            Self::items_table().table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        let result = stmt.query_row(rusqlite::params![item_row_id, site.row_id, key], |_| Ok(()));

        match result {
            Ok(()) => Ok(true),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(e) => Err(SqliteDbError::from(e)),
        }
    }
}

/// Information returned when starting a refresh operation.
#[derive(Debug, Clone)]
pub struct RefreshInfo {
    /// Row ID of the list_metadata record
    pub list_metadata_id: RowId,
    /// New version number (for concurrency checking)
    pub version: i64,
    /// Items per page setting
    pub per_page: i64,
}

/// Information returned when starting a load-next-page operation.
#[derive(Debug, Clone)]
pub struct FetchNextPageInfo {
    /// Row ID of the list_metadata record
    pub list_metadata_id: RowId,
    /// Page number to fetch
    pub page: i64,
    /// Version at start (check before storing results)
    pub version: i64,
    /// Items per page setting
    pub per_page: i64,
}

/// Input for creating a list metadata item.
#[derive(Debug, Clone)]
pub struct ListMetadataItemInput {
    /// Entity ID (post ID, comment ID, etc.)
    pub entity_id: i64,
    /// Last modified timestamp (for staleness detection)
    pub modified_gmt: Option<String>,
    /// Parent entity ID (for hierarchical post types like pages)
    pub parent: Option<i64>,
    /// Menu order (for hierarchical post types)
    pub menu_order: Option<i64>,
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
        let row_id = repo
            .get_or_create(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();

        // Verify it was created with defaults
        let header = repo
            .get_header(&test_ctx.conn, &test_ctx.site, key)
            .unwrap()
            .unwrap();
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
        let first_row_id = repo
            .get_or_create(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();

        // Get or create again should return same rowid
        let second_row_id = repo
            .get_or_create(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();

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
        repo.get_or_create(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();

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
            "SELECT rowid, db_site_id, key, total_pages, total_items, current_page, per_page, last_first_page_fetched_at, last_fetched_at, version FROM {}",
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
            "SELECT rowid, db_site_id, key, entity_id, modified_gmt, parent, menu_order FROM {}",
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
                parent: Some(50),
                menu_order: Some(1),
            },
            ListMetadataItemInput {
                entity_id: 200,
                modified_gmt: Some("2024-01-02T00:00:00Z".to_string()),
                parent: Some(50),
                menu_order: Some(2),
            },
            ListMetadataItemInput {
                entity_id: 300,
                modified_gmt: None,
                parent: None,
                menu_order: None,
            },
        ];

        repo.set_items(&test_ctx.conn, &test_ctx.site, key, &items)
            .unwrap();

        let retrieved = repo.get_items(&test_ctx.conn, &test_ctx.site, key).unwrap();
        assert_eq!(retrieved.len(), 3);
        assert_eq!(retrieved[0].entity_id, 100);
        assert_eq!(retrieved[0].parent, Some(50));
        assert_eq!(retrieved[0].menu_order, Some(1));
        assert_eq!(retrieved[1].entity_id, 200);
        assert_eq!(retrieved[2].entity_id, 300);
        assert!(retrieved[2].modified_gmt.is_none());
        assert!(retrieved[2].parent.is_none());
        assert!(retrieved[2].menu_order.is_none());
    }

    #[rstest]
    fn test_set_items_replaces_existing_items(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:draft";

        // Insert initial items
        let initial_items = vec![
            ListMetadataItemInput {
                entity_id: 1,
                modified_gmt: None,
                parent: None,
                menu_order: None,
            },
            ListMetadataItemInput {
                entity_id: 2,
                modified_gmt: None,
                parent: None,
                menu_order: None,
            },
        ];
        repo.set_items(&test_ctx.conn, &test_ctx.site, key, &initial_items)
            .unwrap();

        // Replace with new items
        let new_items = vec![
            ListMetadataItemInput {
                entity_id: 10,
                modified_gmt: None,
                parent: None,
                menu_order: None,
            },
            ListMetadataItemInput {
                entity_id: 20,
                modified_gmt: None,
                parent: None,
                menu_order: None,
            },
            ListMetadataItemInput {
                entity_id: 30,
                modified_gmt: None,
                parent: None,
                menu_order: None,
            },
        ];
        repo.set_items(&test_ctx.conn, &test_ctx.site, key, &new_items)
            .unwrap();

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
            ListMetadataItemInput {
                entity_id: 1,
                modified_gmt: None,
                parent: None,
                menu_order: None,
            },
            ListMetadataItemInput {
                entity_id: 2,
                modified_gmt: None,
                parent: None,
                menu_order: None,
            },
        ];
        repo.set_items(&test_ctx.conn, &test_ctx.site, key, &initial_items)
            .unwrap();

        // Append more items
        let more_items = vec![
            ListMetadataItemInput {
                entity_id: 3,
                modified_gmt: None,
                parent: None,
                menu_order: None,
            },
            ListMetadataItemInput {
                entity_id: 4,
                modified_gmt: None,
                parent: None,
                menu_order: None,
            },
        ];
        repo.append_items(&test_ctx.conn, &test_ctx.site, key, &more_items)
            .unwrap();

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

        repo.update_header(&test_ctx.conn, &test_ctx.site, key, &update)
            .unwrap();

        let header = repo
            .get_header(&test_ctx.conn, &test_ctx.site, key)
            .unwrap()
            .unwrap();
        assert_eq!(header.total_pages, Some(5));
        assert_eq!(header.total_items, Some(100));
        assert_eq!(header.current_page, 1);
        assert_eq!(header.per_page, 20);
        assert!(header.last_fetched_at.is_some());
    }

    #[rstest]
    fn test_update_state_creates_new_state(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:publish";

        let list_id = repo
            .get_or_create(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();
        repo.update_state(&test_ctx.conn, list_id, ListState::FetchingFirstPage, None)
            .unwrap();

        let state = repo.get_state(&test_ctx.conn, list_id).unwrap().unwrap();
        assert_eq!(state.state, ListState::FetchingFirstPage);
        assert!(state.error_message.is_none());
    }

    #[rstest]
    fn test_update_state_updates_existing_state(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:draft";

        let list_id = repo
            .get_or_create(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();

        // Set initial state
        repo.update_state(&test_ctx.conn, list_id, ListState::FetchingFirstPage, None)
            .unwrap();

        // Update to error state
        repo.update_state(
            &test_ctx.conn,
            list_id,
            ListState::Error,
            Some("Network error"),
        )
        .unwrap();

        let state = repo.get_state(&test_ctx.conn, list_id).unwrap().unwrap();
        assert_eq!(state.state, ListState::Error);
        assert_eq!(state.error_message.as_deref(), Some("Network error"));
    }

    #[rstest]
    fn test_update_state_by_key(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:pending";

        repo.update_state_by_key(
            &test_ctx.conn,
            &test_ctx.site,
            key,
            ListState::FetchingNextPage,
            None,
        )
        .unwrap();

        let state = repo
            .get_state_by_key(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();
        assert_eq!(state, ListState::FetchingNextPage);
    }

    #[rstest]
    fn test_increment_version(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:publish";

        // Create header (version starts at 0)
        repo.get_or_create(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();
        let initial_version = repo
            .get_version(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();
        assert_eq!(initial_version, 0);

        // Increment version
        let new_version = repo
            .increment_version(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();
        assert_eq!(new_version, 1);

        // Increment again
        let newer_version = repo
            .increment_version(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();
        assert_eq!(newer_version, 2);

        // Verify last_first_page_fetched_at is set
        let header = repo
            .get_header(&test_ctx.conn, &test_ctx.site, key)
            .unwrap()
            .unwrap();
        assert!(header.last_first_page_fetched_at.is_some());
    }

    #[rstest]
    fn test_delete_list_removes_all_data(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:publish";

        // Create header and add items and state
        let list_id = repo
            .get_or_create(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();
        let items = vec![ListMetadataItemInput {
            entity_id: 1,
            modified_gmt: None,
            parent: None,
            menu_order: None,
        }];
        repo.set_items(&test_ctx.conn, &test_ctx.site, key, &items)
            .unwrap();
        repo.update_state(&test_ctx.conn, list_id, ListState::Idle, None)
            .unwrap();

        // Verify data exists
        assert!(
            repo.get_header(&test_ctx.conn, &test_ctx.site, key)
                .unwrap()
                .is_some()
        );
        assert_eq!(
            repo.get_item_count(&test_ctx.conn, &test_ctx.site, key)
                .unwrap(),
            1
        );

        // Delete the list
        repo.delete_list(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();

        // Verify everything is deleted
        assert!(
            repo.get_header(&test_ctx.conn, &test_ctx.site, key)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            repo.get_item_count(&test_ctx.conn, &test_ctx.site, key)
                .unwrap(),
            0
        );
    }

    #[rstest]
    fn test_items_preserve_order(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:ordered";

        // Insert items in specific order
        let items: Vec<ListMetadataItemInput> = (1..=10)
            .map(|i| ListMetadataItemInput {
                entity_id: i * 100,
                modified_gmt: None,
                parent: None,
                menu_order: None,
            })
            .collect();

        repo.set_items(&test_ctx.conn, &test_ctx.site, key, &items)
            .unwrap();

        let retrieved = repo.get_items(&test_ctx.conn, &test_ctx.site, key).unwrap();
        assert_eq!(retrieved.len(), 10);

        // Verify order is preserved (rowid ordering)
        for (i, item) in retrieved.iter().enumerate() {
            assert_eq!(item.entity_id, ((i + 1) * 100) as i64);
        }
    }

    // ============================================================
    // Concurrency Helper Tests
    // ============================================================

    #[rstest]
    fn test_begin_refresh_creates_header_and_sets_state(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:publish";

        let info = repo
            .begin_refresh(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();

        // Verify version was incremented (from 0 to 1)
        assert_eq!(info.version, 1);
        assert_eq!(info.per_page, 20); // default

        // Verify state is FetchingFirstPage
        let state = repo
            .get_state_by_key(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();
        assert_eq!(state, ListState::FetchingFirstPage);
    }

    #[rstest]
    fn test_begin_refresh_increments_version_each_time(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:draft";

        let info1 = repo
            .begin_refresh(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();
        assert_eq!(info1.version, 1);

        // Complete the first refresh
        repo.complete_sync(&test_ctx.conn, info1.list_metadata_id)
            .unwrap();

        let info2 = repo
            .begin_refresh(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();
        assert_eq!(info2.version, 2);
    }

    #[rstest]
    fn test_begin_fetch_next_page_returns_none_for_non_existent_list(test_ctx: TestContext) {
        let repo = list_metadata_repo();

        let result = repo
            .begin_fetch_next_page(&test_ctx.conn, &test_ctx.site, "nonexistent")
            .unwrap();
        assert!(result.is_none());
    }

    #[rstest]
    fn test_begin_fetch_next_page_returns_none_when_no_pages_loaded(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:publish";

        // Create header but don't set current_page
        repo.get_or_create(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();

        let result = repo
            .begin_fetch_next_page(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();
        assert!(result.is_none());
    }

    #[rstest]
    fn test_begin_fetch_next_page_returns_none_at_last_page(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:publish";

        // Set up header with current_page = total_pages
        let update = ListMetadataHeaderUpdate {
            total_pages: Some(3),
            total_items: Some(60),
            current_page: 3,
            per_page: 20,
        };
        repo.update_header(&test_ctx.conn, &test_ctx.site, key, &update)
            .unwrap();

        let result = repo
            .begin_fetch_next_page(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();
        assert!(result.is_none());
    }

    #[rstest]
    fn test_begin_fetch_next_page_returns_info_when_more_pages(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:publish";

        // Set up header with more pages available
        let update = ListMetadataHeaderUpdate {
            total_pages: Some(5),
            total_items: Some(100),
            current_page: 2,
            per_page: 20,
        };
        repo.update_header(&test_ctx.conn, &test_ctx.site, key, &update)
            .unwrap();

        let result = repo
            .begin_fetch_next_page(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();
        assert!(result.is_some());

        let info = result.unwrap();
        assert_eq!(info.page, 3); // next page
        assert_eq!(info.per_page, 20);

        // Verify state changed to FetchingNextPage
        let state = repo
            .get_state_by_key(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();
        assert_eq!(state, ListState::FetchingNextPage);
    }

    #[rstest]
    fn test_complete_sync_sets_state_to_idle(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:publish";

        let info = repo
            .begin_refresh(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();
        repo.complete_sync(&test_ctx.conn, info.list_metadata_id)
            .unwrap();

        let state = repo
            .get_state_by_key(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();
        assert_eq!(state, ListState::Idle);
    }

    #[rstest]
    fn test_complete_sync_with_error_sets_state_and_message(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:publish";

        let info = repo
            .begin_refresh(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();
        repo.complete_sync_with_error(&test_ctx.conn, info.list_metadata_id, "Network timeout")
            .unwrap();

        let state_record = repo
            .get_state(&test_ctx.conn, info.list_metadata_id)
            .unwrap()
            .unwrap();
        assert_eq!(state_record.state, ListState::Error);
        assert_eq!(
            state_record.error_message.as_deref(),
            Some("Network timeout")
        );
    }

    #[rstest]
    fn test_version_check_detects_stale_operation(test_ctx: TestContext) {
        let repo = list_metadata_repo();
        let key = "edit:posts:publish";

        // Start a refresh (version becomes 1)
        let refresh_info = repo
            .begin_refresh(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();
        assert_eq!(refresh_info.version, 1);

        // Update header to simulate page 1 loaded
        let update = ListMetadataHeaderUpdate {
            total_pages: Some(5),
            total_items: Some(100),
            current_page: 1,
            per_page: 20,
        };
        repo.update_header(&test_ctx.conn, &test_ctx.site, key, &update)
            .unwrap();
        repo.complete_sync(&test_ctx.conn, refresh_info.list_metadata_id)
            .unwrap();

        // Start load-next-page (captures version = 1)
        let next_page_info = repo
            .begin_fetch_next_page(&test_ctx.conn, &test_ctx.site, key)
            .unwrap()
            .unwrap();
        let captured_version = next_page_info.version;

        // Another refresh happens (version becomes 2)
        repo.begin_refresh(&test_ctx.conn, &test_ctx.site, key)
            .unwrap();

        // Version check should fail (stale)
        let is_valid = repo
            .check_version(&test_ctx.conn, &test_ctx.site, key, captured_version)
            .unwrap();
        assert!(!is_valid, "Version should not match after refresh");
    }
}
