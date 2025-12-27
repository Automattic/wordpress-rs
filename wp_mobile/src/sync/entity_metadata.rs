use wp_api::prelude::WpGmtDateTime;

/// Lightweight metadata for an entity, used for list structure.
///
/// Contains the `id` and optionally `modified_gmt`, which are sufficient
/// to determine list order and detect stale cached entries.
///
/// The `modified_gmt` is optional because some entity types (e.g., Comments)
/// don't have this field. For those entities, staleness is determined via
/// other means (e.g., `last_fetched_at` in the database).
///
/// For hierarchical post types (like pages), `parent` and `menu_order` are
/// also stored to support proper ordering and hierarchy display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityMetadata {
    pub id: i64,
    pub modified_gmt: Option<WpGmtDateTime>,
    /// Parent entity ID (for hierarchical post types like pages)
    pub parent: Option<i64>,
    /// Menu order (for hierarchical post types)
    pub menu_order: Option<i64>,
}

impl EntityMetadata {
    pub fn new(
        id: i64,
        modified_gmt: Option<WpGmtDateTime>,
        parent: Option<i64>,
        menu_order: Option<i64>,
    ) -> Self {
        Self {
            id,
            modified_gmt,
            parent,
            menu_order,
        }
    }

    /// Create metadata with a known modified timestamp.
    pub fn with_modified(id: i64, modified_gmt: WpGmtDateTime) -> Self {
        Self {
            id,
            modified_gmt: Some(modified_gmt),
            parent: None,
            menu_order: None,
        }
    }
}
