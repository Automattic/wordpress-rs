use crate::RowId;

/// Represents list metadata header in the database.
///
/// Stores pagination info and version for a specific list (e.g., "edit:posts:publish").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbListMetadata {
    /// SQLite rowid of this list metadata
    pub row_id: RowId,
    /// Database site ID (rowid from sites table)
    pub db_site_id: RowId,
    /// List key (e.g., "edit:posts:publish")
    pub key: String,
    /// Total number of pages from API response
    pub total_pages: Option<i64>,
    /// Total number of items from API response
    pub total_items: Option<i64>,
    /// Current page that has been loaded (0 = no pages loaded)
    pub current_page: i64,
    /// Items per page
    pub per_page: i64,
    /// ISO 8601 timestamp of when page 1 was last fetched
    pub last_first_page_fetched_at: Option<String>,
    /// ISO 8601 timestamp of last update
    pub last_updated_at: Option<String>,
    /// Version number, incremented on page 1 refresh for concurrency control
    pub version: i64,
}

/// Represents a single item in a list metadata collection.
///
/// Items are ordered by rowid (insertion order = display order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbListMetadataItem {
    /// SQLite rowid (determines display order)
    pub row_id: RowId,
    /// Database site ID
    pub db_site_id: RowId,
    /// List key this item belongs to
    pub key: String,
    /// Entity ID (post ID, comment ID, etc.)
    pub entity_id: i64,
    /// Last modified timestamp (for staleness detection)
    pub modified_gmt: Option<String>,
}

/// Represents sync state for a list metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbListMetadataState {
    /// SQLite rowid
    pub row_id: RowId,
    /// Foreign key to list_metadata.rowid
    pub list_metadata_id: RowId,
    /// Current sync state
    pub state: ListState,
    /// Error message if state is error
    pub error_message: Option<String>,
    /// ISO 8601 timestamp of last state change
    pub updated_at: String,
}

/// Sync state for a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, uniffi::Enum)]
pub enum ListState {
    /// No sync in progress
    #[default]
    Idle,
    /// Fetching first page (pull-to-refresh)
    FetchingFirstPage,
    /// Fetching subsequent page (load more)
    FetchingNextPage,
    /// Last sync failed
    Error,
}

impl ListState {
    /// Convert to database string representation.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            ListState::Idle => "idle",
            ListState::FetchingFirstPage => "fetching_first_page",
            ListState::FetchingNextPage => "fetching_next_page",
            ListState::Error => "error",
        }
    }
}

impl From<&str> for ListState {
    fn from(s: &str) -> Self {
        match s {
            "idle" => ListState::Idle,
            "fetching_first_page" => ListState::FetchingFirstPage,
            "fetching_next_page" => ListState::FetchingNextPage,
            "error" => ListState::Error,
            _ => {
                // Default to Idle for unknown states to avoid panics
                eprintln!("Warning: Unknown ListState '{}', defaulting to Idle", s);
                ListState::Idle
            }
        }
    }
}

impl From<String> for ListState {
    fn from(s: String) -> Self {
        ListState::from(s.as_str())
    }
}
