use std::hash::Hash;
use wp_api::prelude::WpGmtDateTime;

/// Trait for entities that support metadata-based sync.
///
/// Any WordPress REST API entity with `id` and `modified_gmt` fields
/// can implement this trait to work with `MetadataCollection`.
///
/// # Type Parameter
/// - `Id`: The ID type for this entity (e.g., `PostId`, `MediaId`)
///
/// # Example
/// ```ignore
/// impl SyncableEntity for SparseAnyPostWithEditContext {
///     type Id = PostId;
///
///     fn id(&self) -> Option<Self::Id> { self.id }
///     fn modified_gmt(&self) -> Option<&WpGmtDateTime> { self.modified_gmt.as_ref() }
/// }
/// ```
pub trait SyncableEntity {
    /// The ID type for this entity (e.g., `PostId`, `MediaId`)
    type Id: Clone + Eq + Hash + Send + Sync;

    /// Returns the entity's ID, if present.
    fn id(&self) -> Option<Self::Id>;

    /// Returns the entity's modification timestamp in GMT, if present.
    fn modified_gmt(&self) -> Option<&WpGmtDateTime>;
}
