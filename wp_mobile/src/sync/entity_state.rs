/// Fetch state for an entity.
///
/// Tracks the lifecycle of fetching an entity from the network:
/// - `Missing`: Not in cache, needs to be fetched
/// - `Fetching`: Fetch is in progress
/// - `Cached`: Successfully fetched and in cache
/// - `Stale`: In cache but outdated (e.g., `modified_gmt` mismatch)
/// - `Failed`: Fetch was attempted but failed
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
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

    /// Returns `true` if the last fetch attempt failed.
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Create a `Failed` state with the given error message.
    pub fn failed(error: impl Into<String>) -> Self {
        Self::Failed {
            error: error.into(),
        }
    }
}

impl Default for EntityState {
    fn default() -> Self {
        Self::Missing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_fetch() {
        assert!(EntityState::Missing.needs_fetch());
        assert!(EntityState::Stale.needs_fetch());
        assert!(
            EntityState::Failed {
                error: "err".into()
            }
            .needs_fetch()
        );

        assert!(!EntityState::Fetching.needs_fetch());
        assert!(!EntityState::Cached.needs_fetch());
    }

    #[test]
    fn test_is_fetching() {
        assert!(EntityState::Fetching.is_fetching());

        assert!(!EntityState::Missing.is_fetching());
        assert!(!EntityState::Cached.is_fetching());
    }

    #[test]
    fn test_is_cached() {
        assert!(EntityState::Cached.is_cached());
        assert!(EntityState::Stale.is_cached());

        assert!(!EntityState::Missing.is_cached());
        assert!(!EntityState::Fetching.is_cached());
        assert!(
            !EntityState::Failed {
                error: "err".into()
            }
            .is_cached()
        );
    }

    #[test]
    fn test_failed_helper() {
        let state = EntityState::failed("Network error");
        assert!(matches!(state, EntityState::Failed { error } if error == "Network error"));
    }

    #[test]
    fn test_default_is_missing() {
        assert_eq!(EntityState::default(), EntityState::Missing);
    }
}
