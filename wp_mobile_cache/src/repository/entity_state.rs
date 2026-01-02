use crate::{DbTable, SqliteDbError, db_types::db_site::DbSite, repository::QueryExecutor};
use rusqlite::{
    OptionalExtension, params,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput},
};

/// Type-safe identifier for entity collections.
///
/// Each variant represents a specific entity type with its context (e.g., posts in edit context).
/// This prevents arbitrary strings from being passed and ensures only valid entity types are used.
///
/// Stored as INTEGER in the database. The repr(i32) ensures stable values even if the enum
/// definition order changes.
///
/// **IMPORTANT**: Do not change the integer values - they are persisted in the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
#[repr(i32)]
pub enum EntityType {
    /// Posts with edit context (table: posts_edit_context)
    PostsEditContext = 0,
}

impl EntityType {
    /// Get the table name associated with this entity type.
    ///
    /// This is the discriminator used in the entity_state table to separate
    /// different entity types and contexts.
    pub fn table_name(self) -> &'static str {
        match self {
            EntityType::PostsEditContext => "posts_edit_context",
        }
    }
}

impl ToSql for EntityType {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(*self as i32))
    }
}

impl FromSql for EntityType {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> FromSqlResult<Self> {
        i32::column_result(value).and_then(|i| match i {
            0 => Ok(EntityType::PostsEditContext),
            _ => Err(FromSqlError::OutOfRange(i as i64)),
        })
    }
}

/// Database representation of entity state.
///
/// Stored as INTEGER in the database. The repr(i32) ensures stable values
/// even if the enum definition order changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum EntityStateValue {
    Missing = 0,
    Fetching = 1,
    Cached = 2,
    Stale = 3,
    Failed = 4,
}

impl ToSql for EntityStateValue {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(*self as i32))
    }
}

impl FromSql for EntityStateValue {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> FromSqlResult<Self> {
        i32::column_result(value).and_then(|i| match i {
            0 => Ok(EntityStateValue::Missing),
            1 => Ok(EntityStateValue::Fetching),
            2 => Ok(EntityStateValue::Cached),
            3 => Ok(EntityStateValue::Stale),
            4 => Ok(EntityStateValue::Failed),
            _ => Err(FromSqlError::OutOfRange(i as i64)),
        })
    }
}

/// Repository for managing entity state in the database.
///
/// Entity states track the lifecycle of individual entities (posts, categories, etc.)
/// during fetch operations. All functions are stateless.
pub struct EntityStateRepository;

impl EntityStateRepository {
    /// Get the database table for entity state
    pub const fn table() -> DbTable {
        DbTable::EntityState
    }

    /// Get the full table name for entity state
    pub fn table_name() -> &'static str {
        Self::table().table_name()
    }

    // ============================================================
    // Write Operations
    // ============================================================

    /// Set state for an entity.
    ///
    /// Uses UPSERT to handle both insert and update cases.
    /// Updates the timestamp on every change.
    pub fn set_state(
        executor: &impl QueryExecutor,
        entity_id: i64,
        db_site: &DbSite,
        entity_type: EntityType,
        state: EntityStateValue,
        error_message: Option<&str>,
    ) -> Result<(), SqliteDbError> {
        let sql = format!(
            "INSERT INTO {} (entity_id, db_site_id, entity_type, state, error_message, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
             ON CONFLICT(entity_id, db_site_id, entity_type)
             DO UPDATE SET state = ?4, error_message = ?5, updated_at = datetime('now')",
            Self::table_name()
        );
        executor.execute(
            &sql,
            params![
                entity_id,
                db_site.row_id.0,
                entity_type.table_name(),
                state,
                error_message
            ],
        )?;
        Ok(())
    }

    /// Set state for multiple entities (batch operation).
    ///
    /// Uses a single SQL INSERT with multiple VALUES for optimal performance.
    /// Batches are chunked to stay within SQLite's parameter limit (999).
    pub fn set_state_batch(
        executor: &impl QueryExecutor,
        entity_ids: &[i64],
        db_site: &DbSite,
        entity_type: EntityType,
        state: EntityStateValue,
        error_message: Option<&str>,
    ) -> Result<(), SqliteDbError> {
        if entity_ids.is_empty() {
            return Ok(());
        }

        // SQLite has 999 parameter limit. Each row uses 5 params (entity_id, db_site_id,
        // entity_type, state, error_message), so chunk at 199 rows to stay under limit.
        const CHUNK_SIZE: usize = 199;

        for chunk in entity_ids.chunks(CHUNK_SIZE) {
            // Build VALUES clause: (?, ?, ?, ?, ?, datetime('now'))
            let values_placeholders: Vec<String> = (0..chunk.len())
                .map(|_| "(?, ?, ?, ?, ?, datetime('now'))".to_string())
                .collect();
            let values_clause = values_placeholders.join(", ");

            let sql = format!(
                "INSERT INTO {} (entity_id, db_site_id, entity_type, state, error_message, updated_at)
                 VALUES {}
                 ON CONFLICT(entity_id, db_site_id, entity_type)
                 DO UPDATE SET state = excluded.state, error_message = excluded.error_message, updated_at = excluded.updated_at",
                Self::table_name(),
                values_clause
            );

            // Build params: flatten [id, db_site_id, entity_type, state, error_msg] for each entity
            let mut params: Vec<Box<dyn ToSql>> = Vec::with_capacity(chunk.len() * 5);
            for &id in chunk {
                params.push(Box::new(id));
                params.push(Box::new(db_site.row_id.0));
                params.push(Box::new(entity_type.table_name().to_string()));
                params.push(Box::new(state));
                params.push(Box::new(error_message.map(|s| s.to_string())));
            }

            // Convert to &[&dyn ToSql] as required by execute
            let params_refs: Vec<&dyn ToSql> = params.iter().map(|p| p.as_ref()).collect();

            executor.execute(&sql, params_refs.as_slice())?;
        }

        Ok(())
    }

    // ============================================================
    // Read Operations
    // ============================================================

    /// Get state for an entity.
    ///
    /// Returns None if no state exists for the entity.
    pub fn get_state(
        executor: &impl QueryExecutor,
        entity_id: i64,
        db_site: &DbSite,
        entity_type: EntityType,
    ) -> Result<Option<EntityStateValue>, SqliteDbError> {
        let sql = format!(
            "SELECT state FROM {} WHERE entity_id = ?1 AND db_site_id = ?2 AND entity_type = ?3",
            Self::table_name()
        );
        executor
            .prepare(&sql)?
            .query_row(
                params![entity_id, db_site.row_id.0, entity_type.table_name()],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Get error message for an entity.
    ///
    /// Returns None if no state exists or no error message is stored.
    pub fn get_error_message(
        executor: &impl QueryExecutor,
        entity_id: i64,
        db_site: &DbSite,
        entity_type: EntityType,
    ) -> Result<Option<String>, SqliteDbError> {
        let sql = format!(
            "SELECT error_message FROM {} WHERE entity_id = ?1 AND db_site_id = ?2 AND entity_type = ?3",
            Self::table_name()
        );
        executor
            .prepare(&sql)?
            .query_row(
                params![entity_id, db_site.row_id.0, entity_type.table_name()],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    // ============================================================
    // Cleanup Operations
    // ============================================================

    /// Reset entity states on app startup.
    ///
    /// Converts `Fetching` states to `Missing` to prevent stuck loading indicators
    /// while preserving `Cached` states to avoid unnecessary refetching.
    ///
    /// On app restart, all in-flight fetches are abandoned. Rather than clearing
    /// everything, we selectively reset only the incomplete operations to improve
    /// UX by retaining already-fetched data.
    ///
    /// Returns the number of rows updated.
    pub fn reset_states_on_startup(executor: &impl QueryExecutor) -> Result<usize, SqliteDbError> {
        let sql = format!(
            "UPDATE {} SET state = ?1 WHERE state = ?2",
            Self::table_name()
        );
        executor.execute(
            &sql,
            params![EntityStateValue::Missing, EntityStateValue::Fetching],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MigrationManager;
    use rusqlite::Connection;

    use crate::{db_types::self_hosted_site::SelfHostedSite, repository::sites::SiteRepository};

    fn setup_test_db() -> (Connection, DbSite) {
        let mut conn = Connection::open_in_memory().expect("Failed to create in-memory database");
        let mut mgr = MigrationManager::new(&conn).expect("Failed to create MigrationManager");
        mgr.perform_migrations()
            .expect("Failed to perform migrations");

        let site_repo = SiteRepository;
        let self_hosted_site = SelfHostedSite {
            url: "https://test.local".to_string(),
            api_root: "https://test.local/wp-json".to_string(),
        };
        let db_site = site_repo
            .upsert_self_hosted_site(&mut conn, &self_hosted_site)
            .expect("Site creation should succeed")
            .db_site;

        (conn, db_site)
    }

    #[test]
    fn test_upsert_updates_existing_state() {
        let (conn, db_site) = setup_test_db();

        // Insert initial state
        EntityStateRepository::set_state(
            &conn,
            42,
            &db_site,
            EntityType::PostsEditContext,
            EntityStateValue::Fetching,
            None,
        )
        .expect("Failed to set initial state");

        // Update to different state
        EntityStateRepository::set_state(
            &conn,
            42,
            &db_site,
            EntityType::PostsEditContext,
            EntityStateValue::Cached,
            None,
        )
        .expect("Failed to update state");

        let state =
            EntityStateRepository::get_state(&conn, 42, &db_site, EntityType::PostsEditContext)
                .expect("Failed to get state");
        assert_eq!(state, Some(EntityStateValue::Cached));
    }

    #[test]
    fn test_batch_set() {
        let (conn, db_site) = setup_test_db();

        EntityStateRepository::set_state_batch(
            &conn,
            &[1, 2, 3],
            &db_site,
            EntityType::PostsEditContext,
            EntityStateValue::Failed,
            Some("Batch error"),
        )
        .expect("Failed to batch set state");

        assert_eq!(
            EntityStateRepository::get_state(&conn, 1, &db_site, EntityType::PostsEditContext)
                .expect("Failed to get state for entity 1"),
            Some(EntityStateValue::Failed)
        );
        assert_eq!(
            EntityStateRepository::get_state(&conn, 2, &db_site, EntityType::PostsEditContext)
                .expect("Failed to get state for entity 2"),
            Some(EntityStateValue::Failed)
        );
        assert_eq!(
            EntityStateRepository::get_state(&conn, 3, &db_site, EntityType::PostsEditContext)
                .expect("Failed to get state for entity 3"),
            Some(EntityStateValue::Failed)
        );
    }
}
