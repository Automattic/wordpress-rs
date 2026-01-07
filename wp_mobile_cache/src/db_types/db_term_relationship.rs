use crate::{SqliteDbError, term_relationships::DbTermRelationship};
use rusqlite::Row;
use wp_api::terms::TermId;

impl DbTermRelationship {
    /// Construct a term relationship entity from a database row.
    pub fn from_row(row: &Row) -> Result<Self, SqliteDbError> {
        Ok(Self {
            row_id: row.get("rowid")?,
            db_site_id: row.get("db_site_id")?,
            object_id: row.get("object_id")?,
            term_id: TermId(row.get("term_id")?),
            taxonomy_type: row.get::<_, String>("taxonomy_type")?.into(),
        })
    }
}
