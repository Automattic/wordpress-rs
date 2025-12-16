use crate::{
    SqliteDbError,
    db_types::row_ext::{ColumnIndex, RowExt},
    list_metadata::{DbListMetadata, DbListMetadataItem, DbListMetadataState, ListState},
};
use rusqlite::Row;

/// Column indexes for list_metadata table.
/// These must match the order of columns in the CREATE TABLE statement.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub enum ListMetadataColumn {
    Rowid = 0,
    DbSiteId = 1,
    Key = 2,
    TotalPages = 3,
    TotalItems = 4,
    CurrentPage = 5,
    PerPage = 6,
    LastFirstPageFetchedAt = 7,
    LastFetchedAt = 8,
    Version = 9,
}

impl ColumnIndex for ListMetadataColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}

impl DbListMetadata {
    /// Construct a list metadata entity from a database row.
    pub fn from_row(row: &Row) -> Result<Self, SqliteDbError> {
        use ListMetadataColumn as Col;

        Ok(Self {
            row_id: row.get_column(Col::Rowid)?,
            db_site_id: row.get_column(Col::DbSiteId)?,
            key: row.get_column(Col::Key)?,
            total_pages: row.get_column(Col::TotalPages)?,
            total_items: row.get_column(Col::TotalItems)?,
            current_page: row.get_column(Col::CurrentPage)?,
            per_page: row.get_column(Col::PerPage)?,
            last_first_page_fetched_at: row.get_column(Col::LastFirstPageFetchedAt)?,
            last_fetched_at: row.get_column(Col::LastFetchedAt)?,
            version: row.get_column(Col::Version)?,
        })
    }
}

/// Column indexes for list_metadata_items table.
/// These must match the order of columns in the CREATE TABLE statement.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub enum ListMetadataItemColumn {
    Rowid = 0,
    DbSiteId = 1,
    Key = 2,
    EntityId = 3,
    ModifiedGmt = 4,
    Parent = 5,
    MenuOrder = 6,
}

impl ColumnIndex for ListMetadataItemColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}

impl DbListMetadataItem {
    /// Construct a list metadata item from a database row.
    pub fn from_row(row: &Row) -> Result<Self, SqliteDbError> {
        use ListMetadataItemColumn as Col;

        Ok(Self {
            row_id: row.get_column(Col::Rowid)?,
            db_site_id: row.get_column(Col::DbSiteId)?,
            key: row.get_column(Col::Key)?,
            entity_id: row.get_column(Col::EntityId)?,
            modified_gmt: row.get_column(Col::ModifiedGmt)?,
            parent: row.get_column(Col::Parent)?,
            menu_order: row.get_column(Col::MenuOrder)?,
        })
    }
}

/// Column indexes for list_metadata_state table.
/// These must match the order of columns in the CREATE TABLE statement.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub enum ListMetadataStateColumn {
    Rowid = 0,
    ListMetadataId = 1,
    State = 2,
    ErrorMessage = 3,
    UpdatedAt = 4,
}

impl ColumnIndex for ListMetadataStateColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}

impl DbListMetadataState {
    /// Construct a list metadata state from a database row.
    pub fn from_row(row: &Row) -> Result<Self, SqliteDbError> {
        use ListMetadataStateColumn as Col;

        let state_str: String = row.get_column(Col::State)?;

        Ok(Self {
            row_id: row.get_column(Col::Rowid)?,
            list_metadata_id: row.get_column(Col::ListMetadataId)?,
            state: ListState::from(state_str),
            error_message: row.get_column(Col::ErrorMessage)?,
            updated_at: row.get_column(Col::UpdatedAt)?,
        })
    }
}
