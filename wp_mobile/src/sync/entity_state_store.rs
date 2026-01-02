use std::sync::Arc;

use wp_mobile_cache::{
    SqliteDbError, WpApiCache,
    db_types::db_site::DbSite,
    repository::entity_state::{DbEntityState, EntityStateRepository, EntityType},
};

/// Read-only access to entity fetch states.
///
/// This trait allows components (like `MetadataCollection`) to read entity states
/// without being able to modify them. Only the service layer should write states.
pub trait EntityStateReader: Send + Sync {
    /// Get the current state for an entity.
    ///
    /// Returns `DbEntityState::Missing` if the entity has no recorded state.
    fn get(&self, id: i64) -> DbEntityState;
}

/// Stateless service for tracking entity fetch states.
///
/// All functions are associated - callers pass dependencies explicitly.
/// Stores entity states in the database. Database changes trigger UpdateHook
/// notifications automatically (via WpApiCache), ensuring observers receive
/// updates when entities transition between states (e.g., Fetching -> Failed).
///
/// Thread-safe via WpApiCache's internal synchronization.
pub struct EntityStateService;

impl EntityStateService {
    /// Save the state for a single entity to the database.
    ///
    /// Writes to database, triggering UpdateHook notification to observers.
    ///
    /// **Error handling:** Failures are logged but not propagated since entity states
    /// are recalculated on each sync operation.
    pub fn save(
        cache: &WpApiCache,
        db_site: &DbSite,
        entity_type: EntityType,
        id: i64,
        state: DbEntityState,
    ) {
        Self::save_batch(cache, db_site, entity_type, &[id], state);
    }

    /// Save the state for multiple entities to the database (batch operation).
    ///
    /// Uses a single SQL statement to update multiple entities efficiently.
    ///
    /// **Error handling:** Failures are logged but not propagated since entity states
    /// are recalculated on each sync operation.
    pub fn save_batch(
        cache: &WpApiCache,
        db_site: &DbSite,
        entity_type: EntityType,
        ids: &[i64],
        state: DbEntityState,
    ) {
        if let Err(e) = cache.execute(|conn| {
            EntityStateRepository::set_state_batch(conn, ids, db_site, entity_type, &state)
        }) {
            log::warn!(
                "Failed to set entity state for {} ids to {:?}: {} (will be re-evaluated on next sync)",
                ids.len(),
                state,
                e
            );
        }
    }

    /// Get the state for a single entity.
    ///
    /// Returns `DbEntityState::Missing` if the entity has no recorded state.
    pub fn get(
        cache: &WpApiCache,
        db_site: &DbSite,
        entity_type: EntityType,
        id: i64,
    ) -> DbEntityState {
        cache
            .execute(|conn| EntityStateRepository::get_state(conn, id, db_site, entity_type))
            .ok()
            .flatten()
            .unwrap_or(DbEntityState::Missing)
    }

    /// Filter IDs to only those that can be fetched (not currently `Fetching`).
    ///
    /// Returns IDs where state is `Missing`, `Stale`, `Failed`, or not recorded.
    pub fn filter_fetchable(
        cache: &WpApiCache,
        db_site: &DbSite,
        entity_type: EntityType,
        ids: &[i64],
    ) -> Vec<i64> {
        cache
            .execute(|conn| {
                Ok::<Vec<i64>, SqliteDbError>(
                    ids.iter()
                        .filter(|&&id| {
                            match EntityStateRepository::get_state(conn, id, db_site, entity_type) {
                                Ok(Some(state)) => !state.is_fetching(), // Not fetchable if Fetching
                                _ => true, // Everything else is fetchable
                            }
                        })
                        .copied()
                        .collect(),
                )
            })
            .unwrap_or_else(|e| {
                log::warn!(
                    "Failed to check fetchable state for {} IDs, allowing all: {}",
                    ids.len(),
                    e
                );
                ids.to_vec()
            })
    }
}

/// Read-only wrapper for entity state access.
///
/// Implements EntityStateReader trait by delegating to EntityStateService::get().
/// This allows collections to read entity states without being able to modify them.
pub struct EntityStateReaderImpl {
    cache: Arc<WpApiCache>,
    db_site: DbSite,
    entity_type: EntityType,
}

impl EntityStateReaderImpl {
    /// Create a new reader for entity states.
    pub fn new(cache: Arc<WpApiCache>, db_site: DbSite, entity_type: EntityType) -> Self {
        Self {
            cache,
            db_site,
            entity_type,
        }
    }
}

impl EntityStateReader for EntityStateReaderImpl {
    fn get(&self, id: i64) -> DbEntityState {
        EntityStateService::get(&self.cache, &self.db_site, self.entity_type, id)
    }
}

