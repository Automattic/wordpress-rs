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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use wp_mobile_cache::{
        MigrationManager, db_types::self_hosted_site::SelfHostedSite,
        repository::sites::SiteRepository,
    };

    fn setup_test_db() -> (Arc<WpApiCache>, DbSite) {
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

        (Arc::new(WpApiCache::from(conn)), db_site)
    }

    #[test]
    fn test_filter_fetchable() {
        let (cache, db_site) = setup_test_db();

        EntityStateService::save(
            &cache,
            &db_site,
            EntityType::PostsEditContext,
            1,
            DbEntityState::Missing,
        );
        EntityStateService::save(
            &cache,
            &db_site,
            EntityType::PostsEditContext,
            2,
            DbEntityState::Fetching,
        );
        EntityStateService::save(
            &cache,
            &db_site,
            EntityType::PostsEditContext,
            3,
            DbEntityState::Cached,
        );
        EntityStateService::save(
            &cache,
            &db_site,
            EntityType::PostsEditContext,
            4,
            DbEntityState::Stale,
        );
        EntityStateService::save(
            &cache,
            &db_site,
            EntityType::PostsEditContext,
            5,
            DbEntityState::failed("error"),
        );
        // ID 6 has no state (should be fetchable)

        let fetchable = EntityStateService::filter_fetchable(
            &cache,
            &db_site,
            EntityType::PostsEditContext,
            &[1, 2, 3, 4, 5, 6],
        );

        // Only Fetching (2) should be excluded - it's already in progress
        // All others are "fetchable" (not currently being fetched)
        assert!(fetchable.contains(&1)); // Missing
        assert!(!fetchable.contains(&2)); // Fetching - excluded (already in progress)
        assert!(fetchable.contains(&3)); // Cached - fetchable (could re-fetch if needed)
        assert!(fetchable.contains(&4)); // Stale
        assert!(fetchable.contains(&5)); // Failed
        assert!(fetchable.contains(&6)); // Unknown (no state recorded)
    }
}
