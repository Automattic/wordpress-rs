use std::sync::Arc;

use wp_mobile_cache::{
    WpApiCache,
    db_types::db_site::DbSite,
    repository::entity_state::{EntityStateRepository, EntityStateValue, EntityType},
};

use super::EntityState;

/// Read-only access to entity fetch states.
///
/// This trait allows components (like `MetadataCollection`) to read entity states
/// without being able to modify them. Only the service layer should write states.
pub trait EntityStateReader: Send + Sync {
    /// Get the current state for an entity.
    ///
    /// Returns `EntityState::Missing` if the entity has no recorded state.
    fn get(&self, id: i64) -> EntityState;
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
    /// Set the state for a single entity.
    ///
    /// Writes to database, triggering UpdateHook notification to observers.
    /// Failures are logged but not propagated - state writes are best-effort
    /// and will be retried on next sync unless there's a persistent DB issue.
    pub fn set(
        cache: &WpApiCache,
        db_site: &DbSite,
        entity_type: EntityType,
        id: i64,
        state: EntityState,
    ) {
        let (state_value, error_msg) = Self::encode_state(&state);

        if let Err(e) = cache.execute(|conn| {
            EntityStateRepository::set_state(
                conn,
                id,
                db_site,
                entity_type,
                state_value,
                error_msg.as_deref(),
            )
        }) {
            log::warn!(
                "Failed to set entity state for id={} to {:?}: {} (will retry on next sync)",
                id,
                state,
                e
            );
        }
    }

    /// Set the state for multiple entities.
    ///
    /// Writes to database in batch, triggering UpdateHook notification for each entity.
    /// Failures are logged but not propagated - state writes are best-effort
    /// and will be retried on next sync unless there's a persistent DB issue.
    pub fn set_batch(
        cache: &WpApiCache,
        db_site: &DbSite,
        entity_type: EntityType,
        ids: &[i64],
        state: EntityState,
    ) {
        let (state_value, error_msg) = Self::encode_state(&state);

        if let Err(e) = cache.execute(|conn| {
            EntityStateRepository::set_state_batch(
                conn,
                ids,
                db_site,
                entity_type,
                state_value,
                error_msg.as_deref(),
            )
        }) {
            log::warn!(
                "Failed to set entity state for {} ids to {:?}: {} (will retry on next sync)",
                ids.len(),
                state,
                e
            );
        }
    }

    /// Get the state for a single entity.
    ///
    /// Returns `EntityState::Missing` if the entity has no recorded state.
    pub fn get(
        cache: &WpApiCache,
        db_site: &DbSite,
        entity_type: EntityType,
        id: i64,
    ) -> EntityState {
        cache
            .execute(|conn| {
                let state_value = EntityStateRepository::get_state(conn, id, db_site, entity_type)?;
                let error_msg = if state_value == Some(EntityStateValue::Failed) {
                    EntityStateRepository::get_error_message(conn, id, db_site, entity_type)?
                } else {
                    None
                };

                Ok::<EntityState, wp_mobile_cache::SqliteDbError>(match state_value {
                    Some(state) => Self::decode_state(state, error_msg),
                    None => EntityState::Missing,
                })
            })
            .unwrap_or(EntityState::Missing)
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
                Ok::<Vec<i64>, wp_mobile_cache::SqliteDbError>(
                    ids.iter()
                        .filter(|&&id| {
                            match EntityStateRepository::get_state(conn, id, db_site, entity_type) {
                                Ok(Some(EntityStateValue::Fetching)) => false, // Fetching - not fetchable
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

    /// Encode EntityState to (state_value, error_message) for database storage.
    pub fn encode_state(state: &EntityState) -> (EntityStateValue, Option<String>) {
        match state {
            EntityState::Missing => (EntityStateValue::Missing, None),
            EntityState::Fetching => (EntityStateValue::Fetching, None),
            EntityState::Cached => (EntityStateValue::Cached, None),
            EntityState::Stale => (EntityStateValue::Stale, None),
            EntityState::Failed { error } => (EntityStateValue::Failed, Some(error.clone())),
        }
    }

    /// Decode (state_value, error_message) from database to EntityState.
    pub fn decode_state(
        state_value: EntityStateValue,
        error_message: Option<String>,
    ) -> EntityState {
        match state_value {
            EntityStateValue::Missing => EntityState::Missing,
            EntityStateValue::Fetching => EntityState::Fetching,
            EntityStateValue::Cached => EntityState::Cached,
            EntityStateValue::Stale => EntityState::Stale,
            EntityStateValue::Failed => EntityState::Failed {
                error: error_message.unwrap_or_else(|| "Unknown error".to_string()),
            },
        }
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
    fn get(&self, id: i64) -> EntityState {
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

        EntityStateService::set(
            &cache,
            &db_site,
            EntityType::PostsEditContext,
            1,
            EntityState::Missing,
        );
        EntityStateService::set(
            &cache,
            &db_site,
            EntityType::PostsEditContext,
            2,
            EntityState::Fetching,
        );
        EntityStateService::set(
            &cache,
            &db_site,
            EntityType::PostsEditContext,
            3,
            EntityState::Cached,
        );
        EntityStateService::set(
            &cache,
            &db_site,
            EntityType::PostsEditContext,
            4,
            EntityState::Stale,
        );
        EntityStateService::set(
            &cache,
            &db_site,
            EntityType::PostsEditContext,
            5,
            EntityState::failed("error"),
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
