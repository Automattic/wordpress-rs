use crate::{
    SqliteDbError,
    db_types::row_ext::{ColumnIndex, RowExt},
    term_relationships::DbTermRelationship,
};
use rusqlite::Row;
use wp_api::terms::TermId;

/// Column indexes for term_relationships table.
/// These must match the order of columns in the CREATE TABLE statement.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
enum TermRelationshipColumn {
    Rowid = 0,
    DbSiteId = 1,
    ObjectId = 2,
    TermId = 3,
    TaxonomyType = 4,
}

impl ColumnIndex for TermRelationshipColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}

impl DbTermRelationship {
    /// Construct a term relationship entity from a database row.
    pub fn from_row(row: &Row) -> Result<Self, SqliteDbError> {
        use TermRelationshipColumn as Col;

        Ok(Self {
            row_id: row.get_column(Col::Rowid)?,
            db_site_id: row.get_column(Col::DbSiteId)?,
            object_id: row.get_column(Col::ObjectId)?,
            term_id: TermId(row.get_column(Col::TermId)?),
            taxonomy_type: row.get_column::<String, _>(Col::TaxonomyType)?.into(),
        })
    }
}
