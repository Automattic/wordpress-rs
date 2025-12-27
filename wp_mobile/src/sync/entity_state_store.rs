use std::collections::HashMap;
use std::sync::RwLock;

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
/// Maps entity IDs to their current fetch state (Missing, Fetching, Cached, etc.).
/// This is a memory-only store - state resets on app restart.
///
/// Thread-safe via `RwLock<HashMap>`.
pub struct EntityStateStore {
    states: RwLock<HashMap<i64, EntityState>>,
}

impl EntityStateStore {
    pub fn new() -> Self {
        Self {
            states: RwLock::new(HashMap::new()),
        }
    }

    /// Set the state for a single entity.
    pub fn set(&self, id: i64, state: EntityState) {
        self.states
            .write()
            .expect("RwLock poisoned")
            .insert(id, state);
    }

    /// Set the state for multiple entities.
    pub fn set_batch(&self, ids: &[i64], state: EntityState) {
        let mut states = self.states.write().expect("RwLock poisoned");
        ids.iter().for_each(|&id| {
            states.insert(id, state.clone());
        });
    }

    /// Filter IDs to only those that can be fetched (not currently `Fetching`).
    ///
    /// Returns IDs where state is `Missing`, `Stale`, `Failed`, or not recorded.
    pub fn filter_fetchable(&self, ids: &[i64]) -> Vec<i64> {
        let states = self.states.read().expect("RwLock poisoned");
        ids.iter()
            .filter(|&&id| states.get(&id).map(|s| !s.is_fetching()).unwrap_or(true))
            .copied()
            .collect()
    }

    /// Clear all state entries.
    pub fn clear(&self) {
        self.states.write().expect("RwLock poisoned").clear();
    }
}

impl Default for EntityStateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityStateReader for EntityStateStore {
    fn get(&self, id: i64) -> EntityState {
        self.states
            .read()
            .expect("RwLock poisoned")
            .get(&id)
            .cloned()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_fetchable() {
        let store = EntityStateStore::new();

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
