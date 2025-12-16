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

    /// Create metadata without a modified timestamp.
    ///
    /// Use this for entity types that don't have a `modified_gmt` field.
    pub fn without_modified(id: i64) -> Self {
        Self {
            id,
            modified_gmt: None,
            parent: None,
            menu_order: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_with_modified() {
        let modified = WpGmtDateTime::from_timestamp(1000);
        let metadata = EntityMetadata::new(42, Some(modified), Some(10), Some(5));

        assert_eq!(metadata.id, 42);
        assert_eq!(
            metadata.modified_gmt,
            Some(WpGmtDateTime::from_timestamp(1000))
        );
        assert_eq!(metadata.parent, Some(10));
        assert_eq!(metadata.menu_order, Some(5));
    }

    #[test]
    fn test_new_without_modified() {
        let metadata = EntityMetadata::new(42, None, None, None);

        assert_eq!(metadata.id, 42);
        assert_eq!(metadata.modified_gmt, None);
        assert_eq!(metadata.parent, None);
        assert_eq!(metadata.menu_order, None);
    }

    #[test]
    fn test_with_modified_helper() {
        let modified = WpGmtDateTime::from_timestamp(1000);
        let metadata = EntityMetadata::with_modified(42, modified);

        assert_eq!(metadata.id, 42);
        assert!(metadata.modified_gmt.is_some());
        assert!(metadata.parent.is_none());
        assert!(metadata.menu_order.is_none());
    }

    #[test]
    fn test_without_modified_helper() {
        let metadata = EntityMetadata::without_modified(42);

        assert_eq!(metadata.id, 42);
        assert!(metadata.modified_gmt.is_none());
        assert!(metadata.parent.is_none());
        assert!(metadata.menu_order.is_none());
    }
}
