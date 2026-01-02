use crate::{DbTable, RowId, SqliteDbError, repository::QueryExecutor};
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
        db_site_id: RowId,
        entity_type: EntityType,
        state: EntityStateValue,
        error_message: Option<&str>,
    ) -> Result<(), SqliteDbError> {
        let sql = format!(
            "INSERT INTO {} (entity_id, db_site_id, table_name, state, error_message, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
             ON CONFLICT(entity_id, db_site_id, table_name)
             DO UPDATE SET state = ?4, error_message = ?5, updated_at = datetime('now')",
            Self::table_name()
        );
        executor.execute(
            &sql,
            params![
                entity_id,
                db_site_id.0,
                entity_type.table_name(),
                state,
                error_message
            ],
        )?;
        Ok(())
    }

    /// Set state for multiple entities (batch operation).
    ///
    /// Each state write is independent - partial failures are acceptable since entity
    /// states don't require atomicity across batches. If one entity fails to update,
    /// others should still succeed.
    pub fn set_state_batch(
        executor: &impl QueryExecutor,
        entity_ids: &[i64],
        db_site_id: RowId,
        entity_type: EntityType,
        state: EntityStateValue,
        error_message: Option<&str>,
    ) -> Result<(), SqliteDbError> {
        for &id in entity_ids {
            Self::set_state(executor, id, db_site_id, entity_type, state, error_message)?;
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
        db_site_id: RowId,
        entity_type: EntityType,
    ) -> Result<Option<EntityStateValue>, SqliteDbError> {
        let sql = format!(
            "SELECT state FROM {} WHERE entity_id = ?1 AND db_site_id = ?2 AND table_name = ?3",
            Self::table_name()
        );
        executor
            .prepare(&sql)?
            .query_row(
                params![entity_id, db_site_id.0, entity_type.table_name()],
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
        db_site_id: RowId,
        entity_type: EntityType,
    ) -> Result<Option<String>, SqliteDbError> {
        let sql = format!(
            "SELECT error_message FROM {} WHERE entity_id = ?1 AND db_site_id = ?2 AND table_name = ?3",
            Self::table_name()
        );
        executor
            .prepare(&sql)?
            .query_row(
                params![entity_id, db_site_id.0, entity_type.table_name()],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    // ============================================================
    // Cleanup Operations
    // ============================================================

    /// Reset all entity states by deleting all records.
    ///
    /// Entity states are ephemeral - they track in-flight fetch operations.
    /// On app restart, all fetches are complete/abandoned, so reset everything.
    ///
    /// This prevents stuck "Fetching" states and ensures fresh state tracking.
    ///
    /// Returns the number of rows deleted.
    pub fn reset_all_states(executor: &impl QueryExecutor) -> Result<usize, SqliteDbError> {
        let sql = format!("DELETE FROM {}", Self::table_name());
        executor.execute(&sql, params![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MigrationManager;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to create in-memory database");
        let mut mgr = MigrationManager::new(&conn).expect("Failed to create MigrationManager");
        mgr.perform_migrations()
            .expect("Failed to perform migrations");
        conn
    }

    #[test]
    fn test_upsert_updates_existing_state() {
        let conn = setup_test_db();
        let db_site_id = RowId(1);

        // Insert initial state
        EntityStateRepository::set_state(
            &conn,
            42,
            db_site_id,
            EntityType::PostsEditContext,
            EntityStateValue::Fetching,
            None,
        )
        .expect("Failed to set initial state");

        // Update to different state
        EntityStateRepository::set_state(
            &conn,
            42,
            db_site_id,
            EntityType::PostsEditContext,
            EntityStateValue::Cached,
            None,
        )
        .expect("Failed to update state");

        let state =
            EntityStateRepository::get_state(&conn, 42, db_site_id, EntityType::PostsEditContext)
                .expect("Failed to get state");
        assert_eq!(state, Some(EntityStateValue::Cached));
    }

    #[test]
    fn test_reset_all_states() {
        let conn = setup_test_db();
        let db_site_id = RowId(1);

        // Insert some states (using PostsEditContext for all since it's the only variant currently)
        EntityStateRepository::set_state(
            &conn,
            1,
            db_site_id,
            EntityType::PostsEditContext,
            EntityStateValue::Fetching,
            None,
        )
        .expect("Failed to set state for entity 1");
        EntityStateRepository::set_state(
            &conn,
            2,
            db_site_id,
            EntityType::PostsEditContext,
            EntityStateValue::Cached,
            None,
        )
        .expect("Failed to set state for entity 2");
        EntityStateRepository::set_state(
            &conn,
            3,
            db_site_id,
            EntityType::PostsEditContext,
            EntityStateValue::Fetching,
            None,
        )
        .expect("Failed to set state for entity 3");

        // Reset all states
        let count =
            EntityStateRepository::reset_all_states(&conn).expect("Failed to reset all states");
        assert_eq!(count, 3);

        // Verify all gone
        assert_eq!(
            EntityStateRepository::get_state(&conn, 1, db_site_id, EntityType::PostsEditContext)
                .expect("Failed to get state for entity 1"),
            None
        );
        assert_eq!(
            EntityStateRepository::get_state(&conn, 2, db_site_id, EntityType::PostsEditContext)
                .expect("Failed to get state for entity 2"),
            None
        );
        assert_eq!(
            EntityStateRepository::get_state(&conn, 3, db_site_id, EntityType::PostsEditContext)
                .expect("Failed to get state for entity 3"),
            None
        );
    }

    #[test]
    fn test_batch_set() {
        let conn = setup_test_db();
        let db_site_id = RowId(1);

        EntityStateRepository::set_state_batch(
            &conn,
            &[1, 2, 3],
            db_site_id,
            EntityType::PostsEditContext,
            EntityStateValue::Failed,
            Some("Batch error"),
        )
        .expect("Failed to batch set state");

        assert_eq!(
            EntityStateRepository::get_state(&conn, 1, db_site_id, EntityType::PostsEditContext)
                .expect("Failed to get state for entity 1"),
            Some(EntityStateValue::Failed)
        );
        assert_eq!(
            EntityStateRepository::get_state(&conn, 2, db_site_id, EntityType::PostsEditContext)
                .expect("Failed to get state for entity 2"),
            Some(EntityStateValue::Failed)
        );
        assert_eq!(
            EntityStateRepository::get_state(&conn, 3, db_site_id, EntityType::PostsEditContext)
                .expect("Failed to get state for entity 3"),
            Some(EntityStateValue::Failed)
        );
    }
}
