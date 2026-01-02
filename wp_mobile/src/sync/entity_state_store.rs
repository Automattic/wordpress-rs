use std::sync::Arc;

use wp_mobile_cache::{
    RowId, WpApiCache,
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

/// Store for tracking entity fetch states.
///
/// Stores entity states in the database. Database changes trigger UpdateHook
/// notifications automatically (via WpApiCache), ensuring observers receive
/// updates when entities transition between states (e.g., Fetching -> Failed).
///
/// Thread-safe via WpApiCache's internal synchronization.
pub struct EntityStateStore {
    cache: Arc<WpApiCache>,
    db_site_id: RowId,
    entity_type: EntityType,
}

impl EntityStateStore {
    /// Create a new entity state store.
    ///
    /// # Arguments
    /// * `cache` - Database cache for storing states
    /// * `db_site_id` - Database site ID for this store
    /// * `entity_type` - Type-safe entity identifier (e.g., EntityType::PostsEditContext)
    pub fn new(cache: Arc<WpApiCache>, db_site_id: RowId, entity_type: EntityType) -> Self {
        Self {
            cache,
            db_site_id,
            entity_type,
        }
    }

    /// Set the state for a single entity.
    ///
    /// Writes to database, triggering UpdateHook notification to observers.
    pub fn set(&self, id: i64, state: EntityState) {
        let (state_value, error_msg) = Self::encode_state(&state);

        let _ = self.cache.execute(|conn| {
            EntityStateRepository::set_state(
                conn,
                id,
                self.db_site_id,
                self.entity_type,
                state_value,
                error_msg.as_deref(),
            )
        });
    }

    /// Set the state for multiple entities.
    ///
    /// Writes to database in batch, triggering UpdateHook notification for each entity.
    pub fn set_batch(&self, ids: &[i64], state: EntityState) {
        let (state_value, error_msg) = Self::encode_state(&state);

        let _ = self.cache.execute(|conn| {
            EntityStateRepository::set_state_batch(
                conn,
                ids,
                self.db_site_id,
                self.entity_type,
                state_value,
                error_msg.as_deref(),
            )
        });
    }

    /// Filter IDs to only those that can be fetched (not currently `Fetching`).
    ///
    /// Returns IDs where state is `Missing`, `Stale`, `Failed`, or not recorded.
    pub fn filter_fetchable(&self, ids: &[i64]) -> Vec<i64> {
        self.cache
            .execute(|conn| {
                Ok::<Vec<i64>, wp_mobile_cache::SqliteDbError>(
                    ids.iter()
                        .filter(|&&id| {
                            match EntityStateRepository::get_state(
                                conn,
                                id,
                                self.db_site_id,
                                self.entity_type,
                            ) {
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
    fn encode_state(state: &EntityState) -> (EntityStateValue, Option<String>) {
        match state {
            EntityState::Missing => (EntityStateValue::Missing, None),
            EntityState::Fetching => (EntityStateValue::Fetching, None),
            EntityState::Cached => (EntityStateValue::Cached, None),
            EntityState::Stale => (EntityStateValue::Stale, None),
            EntityState::Failed { error } => (EntityStateValue::Failed, Some(error.clone())),
        }
    }

    /// Decode (state_value, error_message) from database to EntityState.
    fn decode_state(state_value: EntityStateValue, error_message: Option<String>) -> EntityState {
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

impl EntityStateReader for EntityStateStore {
    fn get(&self, id: i64) -> EntityState {
        self.cache
            .execute(|conn| {
                let state_value =
                    EntityStateRepository::get_state(conn, id, self.db_site_id, self.entity_type)?;
                let error_msg = if state_value == Some(EntityStateValue::Failed) {
                    EntityStateRepository::get_error_message(
                        conn,
                        id,
                        self.db_site_id,
                        self.entity_type,
                    )?
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_fetchable() {
        let cache = Arc::new(WpApiCache::new(None).expect("Failed to create WpApiCache"));
        cache
            .perform_migrations()
            .expect("Failed to perform migrations");
        let store = EntityStateStore::new(cache, RowId(1), EntityType::PostsEditContext);

        store.set(1, EntityState::Missing);
        store.set(2, EntityState::Fetching);
        store.set(3, EntityState::Cached);
        store.set(4, EntityState::Stale);
        store.set(5, EntityState::failed("error"));
        // ID 6 has no state (should be fetchable)

        let fetchable = store.filter_fetchable(&[1, 2, 3, 4, 5, 6]);

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
