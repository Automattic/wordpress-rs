use std::hash::Hash;
use wp_api::prelude::WpGmtDateTime;

/// Lightweight metadata for an entity, used for list structure.
///
/// Contains only the `id` and `modified_gmt` fields, which are sufficient
/// to determine list order and detect stale cached entries.
///
/// # Type Parameter
/// - `Id`: The ID type for the entity (e.g., `PostId`, `MediaId`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityMetadata<Id>
where
    Id: Clone + Eq + Hash,
{
    pub id: Id,
    pub modified_gmt: WpGmtDateTime,
}

impl<Id> EntityMetadata<Id>
where
    Id: Clone + Eq + Hash,
{
    pub fn new(id: Id, modified_gmt: WpGmtDateTime) -> Self {
        Self { id, modified_gmt }
    }
}
