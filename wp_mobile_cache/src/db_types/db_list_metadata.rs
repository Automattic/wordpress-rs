use crate::{
    SqliteDbError,
    list_metadata::{
        DbListHeaderWithState, DbListMetadata, DbListMetadataItem, DbListMetadataState, ListState,
    },
};
use rusqlite::Row;

impl DbListMetadata {
    /// Construct a list metadata entity from a database row.
    pub fn from_row(row: &Row) -> Result<Self, SqliteDbError> {
        Ok(Self {
            row_id: row.get("rowid")?,
            db_site_id: row.get("db_site_id")?,
            key: row.get("key")?,
            total_pages: row.get("total_pages")?,
            total_items: row.get("total_items")?,
            current_page: row.get("current_page")?,
            per_page: row.get("per_page")?,
            last_first_page_fetched_at: row.get("last_first_page_fetched_at")?,
            last_fetched_at: row.get("last_fetched_at")?,
            version: row.get("version")?,
        })
    }
}

impl DbListMetadataItem {
    /// Construct a list metadata item from a database row.
    pub fn from_row(row: &Row) -> Result<Self, SqliteDbError> {
        Ok(Self {
            row_id: row.get("rowid")?,
            list_metadata_id: row.get("list_metadata_id")?,
            entity_id: row.get("entity_id")?,
            modified_gmt: row.get("modified_gmt")?,
            parent: row.get("parent")?,
            menu_order: row.get("menu_order")?,
        })
    }
}

impl DbListMetadataState {
    /// Construct a list metadata state from a database row.
    pub fn from_row(row: &Row) -> Result<Self, SqliteDbError> {
        Ok(Self {
            row_id: row.get("rowid")?,
            list_metadata_id: row.get("list_metadata_id")?,
            state: row.get("state")?,
            error_message: row.get("error_message")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

impl DbListHeaderWithState {
    /// Construct from a JOIN query row.
    ///
    /// Expects columns in order: total_pages, total_items, current_page, per_page, state, error_message
    pub fn from_row(row: &Row) -> Result<Self, SqliteDbError> {
        // state is nullable due to LEFT JOIN - default to Idle
        let state: Option<ListState> = row.get("state")?;

        Ok(Self {
            state: state.unwrap_or(ListState::Idle),
            error_message: row.get("error_message")?,
            current_page: row.get("current_page")?,
            total_pages: row.get("total_pages")?,
            total_items: row.get("total_items")?,
            per_page: row.get("per_page")?,
        })
    }
}
