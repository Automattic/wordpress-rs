/// Result of a sync operation (refresh or load_next_page).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SyncResult {
    /// Number of items in the list after sync.
    pub total_items: u64,

    /// Number of items that were fetched (missing + stale).
    pub fetched_count: u64,

    /// Number of items that failed to fetch.
    pub failed_count: u64,

    /// Whether there are more pages available.
    pub has_more_pages: bool,

    /// Current page number after sync.
    pub current_page: u32,

    /// Total number of pages, if known.
    #[uniffi(default = None)]
    pub total_pages: Option<u32>,
}

impl SyncResult {
    pub fn new(
        total_items: usize,
        fetched_count: usize,
        failed_count: usize,
        has_more_pages: bool,
        current_page: u32,
        total_pages: Option<u32>,
    ) -> Self {
        Self {
            total_items: total_items as u64,
            fetched_count: fetched_count as u64,
            failed_count: failed_count as u64,
            has_more_pages,
            current_page,
            total_pages,
        }
    }

    /// Create a result indicating no sync was needed.
    pub fn no_op(
        total_items: usize,
        has_more_pages: bool,
        current_page: u32,
        total_pages: Option<u32>,
    ) -> Self {
        Self {
            total_items: total_items as u64,
            fetched_count: 0,
            failed_count: 0,
            has_more_pages,
            current_page,
            total_pages,
        }
    }

    /// Returns `true` if all requested fetches succeeded.
    pub fn all_succeeded(&self) -> bool {
        self.failed_count == 0
    }

    /// Returns `true` if some fetches failed.
    pub fn has_failures(&self) -> bool {
        self.failed_count > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let result = SyncResult::new(10, 3, 1, true, 2, Some(5));

        assert_eq!(result.total_items, 10);
        assert_eq!(result.fetched_count, 3);
        assert_eq!(result.failed_count, 1);
        assert!(result.has_more_pages);
        assert_eq!(result.current_page, 2);
        assert_eq!(result.total_pages, Some(5));
    }

    #[test]
    fn test_no_op() {
        let result = SyncResult::no_op(5, false, 3, Some(3));

        assert_eq!(result.total_items, 5);
        assert_eq!(result.fetched_count, 0);
        assert_eq!(result.failed_count, 0);
        assert!(!result.has_more_pages);
        assert_eq!(result.current_page, 3);
        assert_eq!(result.total_pages, Some(3));
    }

    #[test]
    fn test_success_helpers() {
        let success = SyncResult::new(10, 5, 0, true, 1, Some(2));
        assert!(success.all_succeeded());
        assert!(!success.has_failures());

        let partial = SyncResult::new(10, 5, 2, true, 1, Some(2));
        assert!(!partial.all_succeeded());
        assert!(partial.has_failures());
    }
}
