use std::hash::Hash;
use wp_mobile_cache::entity::FullEntity;

use super::EntityMetadata;

/// An item in an entity list - loaded, loading, or failed.
///
/// Used by `MetadataCollection` to represent list items where some entities
/// may still be loading from the network or may have failed to load.
///
/// # Type Parameters
/// - `T`: The full entity type (e.g., `AnyPostWithEditContext`)
/// - `Id`: The ID type (e.g., `PostId`)
#[derive(Debug, Clone)]
pub enum ListItem<T, Id>
where
    Id: Clone + Eq + Hash,
{
    /// Fully loaded entity from cache
    Loaded(FullEntity<T>),

    /// Placeholder for an entity being fetched.
    /// Contains the metadata so we know the ID and modification time.
    Loading(EntityMetadata<Id>),

    /// Entity failed to load.
    /// Contains the metadata for retry purposes and an error message.
    Failed {
        metadata: EntityMetadata<Id>,
        error: String,
    },
}

impl<T, Id> ListItem<T, Id>
where
    Id: Clone + Eq + Hash,
{
    /// Returns `true` if this item is loaded.
    pub fn is_loaded(&self) -> bool {
        matches!(self, ListItem::Loaded(_))
    }

    /// Returns `true` if this item is still loading.
    pub fn is_loading(&self) -> bool {
        matches!(self, ListItem::Loading(_))
    }

    /// Returns `true` if this item failed to load.
    pub fn is_failed(&self) -> bool {
        matches!(self, ListItem::Failed { .. })
    }

    /// Returns the loaded entity, if available.
    pub fn as_loaded(&self) -> Option<&FullEntity<T>> {
        match self {
            ListItem::Loaded(entity) => Some(entity),
            _ => None,
        }
    }

    /// Returns the metadata, if loading or failed.
    pub fn metadata(&self) -> Option<&EntityMetadata<Id>> {
        match self {
            ListItem::Loaded(_) => None,
            ListItem::Loading(metadata) => Some(metadata),
            ListItem::Failed { metadata, .. } => Some(metadata),
        }
    }

    /// Returns the ID of the item, regardless of load state.
    pub fn id(&self) -> &Id
    where
        T: HasId<Id>,
    {
        match self {
            ListItem::Loaded(entity) => entity.data.id(),
            ListItem::Loading(metadata) => &metadata.id,
            ListItem::Failed { metadata, .. } => &metadata.id,
        }
    }
}

/// Helper trait for entities that have an ID field.
///
/// This is used by `ListItem::id()` to extract the ID from loaded entities.
pub trait HasId<Id> {
    fn id(&self) -> &Id;
}
