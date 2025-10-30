use crate::{RowId, db_types::row_ext::ColumnIndex};

/// Column indexes for self_hosted_sites table.
/// These must match the order of columns in the CREATE TABLE statement.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum SelfHostedSiteColumn {
    Rowid = 0,
    Url = 1,
    ApiRoot = 2,
}

impl ColumnIndex for SelfHostedSiteColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}

/// Represents a self-hosted WordPress site in the database.
pub struct DbSelfHostedSite {
    pub row_id: RowId,
    pub url: String,
    pub api_root: String,
}
