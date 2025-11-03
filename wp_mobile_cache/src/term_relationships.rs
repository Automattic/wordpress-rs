use crate::RowId;
use wp_api::taxonomies::TaxonomyType;
use wp_api::terms::TermId;

/// Represents a term relationship in the database.
///
/// This associates an object (post, page, nav item, etc.) with a WordPress term
/// for a specific taxonomy (category, tag, custom taxonomy).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbTermRelationship {
    /// SQLite rowid of this relationship
    pub row_id: RowId,
    /// Database site ID (rowid from sites table) this relationship belongs to
    pub db_site_id: RowId,
    /// Row ID of the object (post, page, etc.) in its respective table
    pub object_id: RowId,
    /// WordPress term ID
    pub term_id: TermId,
    /// Taxonomy type (category, post_tag, or custom)
    pub taxonomy_type: TaxonomyType,
}
