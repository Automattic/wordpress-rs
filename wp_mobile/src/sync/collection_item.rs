use super::{EntityMetadata, EntityState};

/// An item in a metadata-driven collection.
///
/// Combines the lightweight metadata (id + modified_gmt) with the current
/// fetch state. Platform layers wrap this as observable, with `loadData()`
/// fetching the full entity from cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionItem {
    /// Lightweight metadata for this item.
    pub metadata: EntityMetadata,

    /// Current fetch state.
    pub state: EntityState,
}

impl CollectionItem {
    pub fn new(metadata: EntityMetadata, state: EntityState) -> Self {
        Self { metadata, state }
    }

    /// The entity ID.
    pub fn id(&self) -> i64 {
        self.metadata.id
    }

    /// Returns `true` if the entity needs to be fetched.
    pub fn needs_fetch(&self) -> bool {
        self.state.needs_fetch()
    }

    /// Returns `true` if a fetch is currently in progress.
    pub fn is_fetching(&self) -> bool {
        self.state.is_fetching()
    }

    /// Returns `true` if the entity is cached (fresh or stale).
    pub fn is_cached(&self) -> bool {
        self.state.is_cached()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wp_api::prelude::WpGmtDateTime;

    fn test_metadata() -> EntityMetadata {
        EntityMetadata::with_modified(42, WpGmtDateTime::from_timestamp(1000))
    }

    #[test]
    fn test_new() {
        let item = CollectionItem::new(test_metadata(), EntityState::Cached);

        assert_eq!(item.id(), 42);
        assert_eq!(item.state, EntityState::Cached);
    }

    #[test]
    fn test_delegates_to_state() {
        let missing = CollectionItem::new(test_metadata(), EntityState::Missing);
        assert!(missing.needs_fetch());
        assert!(!missing.is_fetching());
        assert!(!missing.is_cached());

        let fetching = CollectionItem::new(test_metadata(), EntityState::Fetching);
        assert!(!fetching.needs_fetch());
        assert!(fetching.is_fetching());
        assert!(!fetching.is_cached());

        let cached = CollectionItem::new(test_metadata(), EntityState::Cached);
        assert!(!cached.needs_fetch());
        assert!(!cached.is_fetching());
        assert!(cached.is_cached());
    }
}
