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
}
