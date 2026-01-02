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
/// Stored as INTEGER in the database via ToSql/FromSql implementations. The repr(i32) ensures
/// stable values even if the enum definition order changes.
///
/// **IMPORTANT**: Do not change the integer values - they are persisted in the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
#[repr(i32)]
pub enum EntityType {
    /// Posts with edit context (table: posts_edit_context)
    PostsEditContext = 0,
}

impl EntityType {
    /// Get a human-readable identifier for this entity type.
    ///
    /// Used for logging and debugging. The database stores the integer value
    /// from the enum, not this string.
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

/// Fetch state for an entity.
///
/// Tracks the lifecycle of fetching an entity from the network:
/// - `Missing`: Not in cache, needs to be fetched
/// - `Fetching`: Fetch is in progress
/// - `Cached`: Successfully fetched and in cache
/// - `Stale`: In cache but outdated (e.g., `modified_gmt` mismatch)
/// - `Failed`: Fetch was attempted but failed
///
/// Stored in database as (state INTEGER, error_message TEXT).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityState {
    /// Entity is not in cache and not being fetched.
    Missing,

    /// Fetch is currently in progress.
    Fetching,

    /// Entity is in cache and considered fresh.
    Cached,

    /// Entity is in cache but outdated (needs re-fetch).
    Stale,

    /// Fetch was attempted but failed.
    Failed { error: String },
}

impl EntityState {
    /// Returns `true` if the entity needs to be fetched.
    ///
    /// This includes `Missing`, `Stale`, and `Failed` states.
    /// Does not include `Fetching` (already in progress) or `Cached` (up to date).
    pub fn needs_fetch(&self) -> bool {
        matches!(self, Self::Missing | Self::Stale | Self::Failed { .. })
    }

    /// Returns `true` if a fetch is currently in progress.
    pub fn is_fetching(&self) -> bool {
        matches!(self, Self::Fetching)
    }

    /// Returns `true` if the entity is cached (fresh or stale).
    pub fn is_cached(&self) -> bool {
        matches!(self, Self::Cached | Self::Stale)
    }

    /// Create a `Failed` state with the given error message.
    pub fn failed(error: impl Into<String>) -> Self {
        Self::Failed {
            error: error.into(),
        }
    }

    /// Encode EntityState to (state_int, error_message) for database storage.
    fn to_db_representation(&self) -> (i32, Option<String>) {
        match self {
            Self::Missing => (0, None),
            Self::Fetching => (1, None),
            Self::Cached => (2, None),
            Self::Stale => (3, None),
            Self::Failed { error } => (4, Some(error.clone())),
        }
    }

    /// Decode (state_int, error_message) from database to EntityState.
    fn from_db_representation(state_int: i32, error_message: Option<String>) -> Option<Self> {
        match state_int {
            0 => Some(Self::Missing),
            1 => Some(Self::Fetching),
            2 => Some(Self::Cached),
            3 => Some(Self::Stale),
            4 => Some(Self::Failed {
                error: error_message.unwrap_or_else(|| "Unknown error".to_string()),
            }),
            _ => None,
        }
    }
}

impl Default for EntityState {
    fn default() -> Self {
        Self::Missing
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
        state: &EntityState,
    ) -> Result<(), SqliteDbError> {
        let (state_int, error_message) = state.to_db_representation();
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
                entity_type,
                state_int,
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
        state: &EntityState,
    ) -> Result<(), SqliteDbError> {
        if entity_ids.is_empty() {
            return Ok(());
        }

        let (state_int, error_message) = state.to_db_representation();

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
                params.push(Box::new(entity_type));
                params.push(Box::new(state_int));
                params.push(Box::new(error_message.clone()));
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
    ) -> Result<Option<EntityState>, SqliteDbError> {
        let sql = format!(
            "SELECT state, error_message FROM {} WHERE entity_id = ?1 AND db_site_id = ?2 AND entity_type = ?3",
            Self::table_name()
        );
        let result = executor
            .prepare(&sql)?
            .query_row(
                params![entity_id, db_site.row_id.0, entity_type],
                |row| {
                    let state_int: i32 = row.get(0)?;
                    let error_message: Option<String> = row.get(1)?;
                    Ok((state_int, error_message))
                },
            )
            .optional()?;

        match result {
            None => Ok(None),
            Some((state_int, error_message)) => {
                EntityState::from_db_representation(state_int, error_message)
                    .map(Some)
                    .ok_or_else(|| {
                        SqliteDbError::SqliteError(format!(
                            "Invalid entity state value {} in database",
                            state_int
                        ))
                    })
            }
        }
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
        let (missing_state, _) = EntityState::Missing.to_db_representation();
        let (fetching_state, _) = EntityState::Fetching.to_db_representation();

        let sql = format!(
            "UPDATE {} SET state = ?1 WHERE state = ?2",
            Self::table_name()
        );
        executor.execute(&sql, params![missing_state, fetching_state])
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
            &EntityState::Fetching,
        )
        .expect("Failed to set initial state");

        // Update to different state
        EntityStateRepository::set_state(
            &conn,
            42,
            &db_site,
            EntityType::PostsEditContext,
            &EntityState::Cached,
        )
        .expect("Failed to update state");

        let state =
            EntityStateRepository::get_state(&conn, 42, &db_site, EntityType::PostsEditContext)
                .expect("Failed to get state");
        assert_eq!(state, Some(EntityState::Cached));
    }

    #[test]
    fn test_batch_set() {
        let (conn, db_site) = setup_test_db();

        EntityStateRepository::set_state_batch(
            &conn,
            &[1, 2, 3],
            &db_site,
            EntityType::PostsEditContext,
            &EntityState::Failed {
                error: "Batch error".to_string(),
            },
        )
        .expect("Failed to batch set state");

        assert_eq!(
            EntityStateRepository::get_state(&conn, 1, &db_site, EntityType::PostsEditContext)
                .expect("Failed to get state for entity 1"),
            Some(EntityState::Failed {
                error: "Batch error".to_string()
            })
        );
        assert_eq!(
            EntityStateRepository::get_state(&conn, 2, &db_site, EntityType::PostsEditContext)
                .expect("Failed to get state for entity 2"),
            Some(EntityState::Failed {
                error: "Batch error".to_string()
            })
        );
        assert_eq!(
            EntityStateRepository::get_state(&conn, 3, &db_site, EntityType::PostsEditContext)
                .expect("Failed to get state for entity 3"),
            Some(EntityState::Failed {
                error: "Batch error".to_string()
            })
        );
    }
}
