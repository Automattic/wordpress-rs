use super::EntityMetadata;

/// Result of a metadata fetch operation.
///
/// Contains lightweight metadata (id + modified_gmt) for entities,
/// plus pagination info from the API response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataFetchResult {
    /// Metadata for entities in this page.
    pub metadata: Vec<EntityMetadata>,

    /// Total number of items matching the query (from API headers).
    pub total_items: Option<i64>,

    /// Total number of pages available (from API headers).
    pub total_pages: Option<u32>,

    /// The page number that was fetched.
    pub current_page: u32,
}

impl MetadataFetchResult {
    pub fn new(
        metadata: Vec<EntityMetadata>,
        total_items: Option<i64>,
        total_pages: Option<u32>,
        current_page: u32,
    ) -> Self {
        Self {
            metadata,
            total_items,
            total_pages,
            current_page,
        }
    }

    /// Returns `true` if there are more pages after this one.
    pub fn has_more_pages(&self) -> bool {
        self.total_pages
            .map(|total| self.current_page < total)
            .unwrap_or(false)
    }

    /// Returns the number of items in this page.
    pub fn page_count(&self) -> usize {
        self.metadata.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wp_api::prelude::WpGmtDateTime;

    fn test_metadata(id: i64) -> EntityMetadata {
        EntityMetadata::with_modified(id, WpGmtDateTime::from_timestamp(1000 + id))
    }

    #[test]
    fn test_new() {
        let result = MetadataFetchResult::new(
            vec![test_metadata(1), test_metadata(2)],
            Some(50),
            Some(5),
            1,
        );

        assert_eq!(result.page_count(), 2);
        assert_eq!(result.total_items, Some(50));
        assert_eq!(result.total_pages, Some(5));
        assert_eq!(result.current_page, 1);
    }

    #[test]
    fn test_has_more_pages() {
        let page_1_of_3 = MetadataFetchResult::new(vec![], None, Some(3), 1);
        assert!(page_1_of_3.has_more_pages());

        let page_3_of_3 = MetadataFetchResult::new(vec![], None, Some(3), 3);
        assert!(!page_3_of_3.has_more_pages());

        let unknown_total = MetadataFetchResult::new(vec![], None, None, 1);
        assert!(!unknown_total.has_more_pages());
    }
}
