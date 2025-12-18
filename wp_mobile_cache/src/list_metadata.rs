use crate::RowId;
use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput};
use std::fmt;

/// Type-safe wrapper for list keys.
///
/// List keys identify specific lists, e.g., `"edit:posts:publish"` or `"view:comments"`.
/// Using a newtype prevents accidental misuse of arbitrary strings as keys.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ListKey(String);

impl ListKey {
    /// Create a new ListKey from a string.
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// Get the key as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ListKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ListKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ListKey {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for ListKey {
    fn from(s: String) -> Self {
        Self(s)
    }
}

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
    /// ISO 8601 timestamp of when any page was last fetched
    pub last_fetched_at: Option<String>,
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
    /// Foreign key to list_metadata table
    pub list_metadata_id: RowId,
    /// Entity ID (post ID, comment ID, etc.)
    pub entity_id: i64,
    /// Last modified timestamp (for staleness detection)
    pub modified_gmt: Option<String>,
    /// Parent entity ID (for hierarchical post types like pages)
    pub parent: Option<i64>,
    /// Menu order (for hierarchical post types)
    pub menu_order: Option<i64>,
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
///
/// Stored as INTEGER in the database. The repr(i32) ensures stable values
/// even if the enum definition order changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, uniffi::Enum)]
#[repr(i32)]
pub enum ListState {
    /// No sync in progress
    #[default]
    Idle = 0,
    /// Fetching first page (pull-to-refresh)
    FetchingFirstPage = 1,
    /// Fetching subsequent page (load more)
    FetchingNextPage = 2,
    /// Last sync failed
    Error = 3,
}

impl ToSql for ListState {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(*self as i32))
    }
}

impl FromSql for ListState {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> FromSqlResult<Self> {
        i32::column_result(value).and_then(|i| match i {
            0 => Ok(ListState::Idle),
            1 => Ok(ListState::FetchingFirstPage),
            2 => Ok(ListState::FetchingNextPage),
            3 => Ok(ListState::Error),
            _ => Err(rusqlite::types::FromSqlError::Other(
                format!("Invalid ListState value: {}", i).into(),
            )),
        })
    }
}

/// Combined header + state from a JOIN query.
///
/// Contains pagination info from `list_metadata` and sync state from `list_metadata_state`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbListHeaderWithState {
    /// Current sync state (defaults to Idle if no state record exists)
    pub state: ListState,
    /// Error message if state is Error
    pub error_message: Option<String>,
    /// Current page that has been loaded (0 = no pages loaded)
    pub current_page: i64,
    /// Total number of pages from API response
    pub total_pages: Option<i64>,
    /// Total number of items from API response
    pub total_items: Option<i64>,
    /// Items per page
    pub per_page: i64,
}
