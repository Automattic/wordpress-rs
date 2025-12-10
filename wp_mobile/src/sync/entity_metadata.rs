use wp_api::prelude::WpGmtDateTime;

/// Lightweight metadata for an entity, used for list structure.
///
/// Contains the `id` and optionally `modified_gmt`, which are sufficient
/// to determine list order and detect stale cached entries.
///
/// The `modified_gmt` is optional because some entity types (e.g., Comments)
/// don't have this field. For those entities, staleness is determined via
/// other means (e.g., `last_fetched_at` in the database).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityMetadata {
    pub id: i64,
    pub modified_gmt: Option<WpGmtDateTime>,
}

impl EntityMetadata {
    pub fn new(id: i64, modified_gmt: Option<WpGmtDateTime>) -> Self {
        Self { id, modified_gmt }
    }

    /// Create metadata with a known modified timestamp.
    pub fn with_modified(id: i64, modified_gmt: WpGmtDateTime) -> Self {
        Self {
            id,
            modified_gmt: Some(modified_gmt),
        }
    }

    /// Create metadata without a modified timestamp.
    ///
    /// Use this for entity types that don't have a `modified_gmt` field.
    pub fn without_modified(id: i64) -> Self {
        Self {
            id,
            modified_gmt: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_with_modified() {
        let modified = WpGmtDateTime::from_timestamp(1000);
        let metadata = EntityMetadata::new(42, Some(modified));

        assert_eq!(metadata.id, 42);
        assert_eq!(
            metadata.modified_gmt,
            Some(WpGmtDateTime::from_timestamp(1000))
        );
    }

    #[test]
    fn test_new_without_modified() {
        let metadata = EntityMetadata::new(42, None);

        assert_eq!(metadata.id, 42);
        assert_eq!(metadata.modified_gmt, None);
    }

    #[test]
    fn test_with_modified_helper() {
        let modified = WpGmtDateTime::from_timestamp(1000);
        let metadata = EntityMetadata::with_modified(42, modified);

        assert_eq!(metadata.id, 42);
        assert!(metadata.modified_gmt.is_some());
    }

    #[test]
    fn test_without_modified_helper() {
        let metadata = EntityMetadata::without_modified(42);

        assert_eq!(metadata.id, 42);
        assert!(metadata.modified_gmt.is_none());
    }
}
