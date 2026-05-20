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
    ///
    /// - `None` - Unknown (no metadata loaded or total_pages not provided by API)
    /// - `Some(true)` - More pages available
    /// - `Some(false)` - On last page
    #[uniffi(default = None)]
    pub has_more_pages: Option<bool>,

    /// Current page number after sync.
    ///
    /// `None` means no pages have been loaded yet.
    #[uniffi(default = None)]
    pub current_page: Option<u32>,

    /// Total number of pages, if known.
    #[uniffi(default = None)]
    pub total_pages: Option<u32>,
}

impl SyncResult {
    pub fn new(
        total_items: u32,
        fetched_count: u32,
        failed_count: u32,
        has_more_pages: Option<bool>,
        current_page: Option<u32>,
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
        total_items: u32,
        has_more_pages: Option<bool>,
        current_page: Option<u32>,
        total_pages: Option<u32>,
    ) -> Self {
        Self {
            total_items: total_items.into(),
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
