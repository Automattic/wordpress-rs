use crate::{RowId, db_types::db_site::DbSite};

/// Unique identifier for an entity stored in the cache database
///
/// Encapsulates the complete identity of a cached entity:
/// - Which site (DbSite with site_type and mapped_site_id)
/// - Which table (table_name)
/// - Which row (rowid)
///
/// This type serves as an opaque handle that can be used to:
/// - Create observable entities without database lookups
/// - Compare entities for identity equality
/// - Filter database change notifications
/// - Reload entity data via load_data() using the stored DbSite
///
/// EntityId is immutable once created and remains valid even if the
/// underlying database row is deleted (though queries may return None).
#[derive(uniffi::Object, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntityId {
    /// The site this entity belongs to
    db_site: DbSite,

    /// The table name where this entity is stored (e.g., "posts_edit_context")
    table_name: String,

    /// The database rowid (SQLite autoincrement primary key)
    rowid: RowId,
}

#[uniffi::export]
impl EntityId {
    /// Check if two EntityIds refer to the same database entity
    ///
    /// Two EntityIds are considered the same if they have matching
    /// db_site, table_name, and rowid.
    pub fn is_same_entity(&self, other: &EntityId) -> bool {
        self.db_site == other.db_site
            && self.table_name == other.table_name
            && self.rowid == other.rowid
    }
}

impl EntityId {
    /// Create a new EntityId (internal only - not exposed via UniFFI)
    pub(crate) fn new(db_site: DbSite, table_name: String, rowid: RowId) -> Self {
        Self {
            db_site,
            table_name,
            rowid,
        }
    }

    /// Get the rowid (internal only)
    pub(crate) fn rowid(&self) -> RowId {
        self.rowid
    }

    /// Get the table name (internal only)
    pub(crate) fn table_name(&self) -> &str {
        &self.table_name
    }

    /// Get the DbSite (internal only)
    pub(crate) fn db_site(&self) -> &DbSite {
        &self.db_site
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RowId, db_types::db_site::DbSiteType};

    /// Helper to create a DbSite for testing.
    /// Uses the same value for both row_id and mapped_site_id for convenience.
    /// In real data, these would be different (row_id is from db_sites table,
    /// mapped_site_id is from the type-specific table like self_hosted_sites).
    fn make_db_site(id: u64) -> DbSite {
        DbSite {
            row_id: RowId(id),
            site_type: DbSiteType::SelfHosted,
            mapped_site_id: RowId(id),
        }
    }

    #[test]
    fn test_is_same_entity_matching() {
        let site = make_db_site(1);
        let id1 = EntityId::new(site.clone(), "posts_edit_context".to_string(), RowId(42));
        let id2 = EntityId::new(site, "posts_edit_context".to_string(), RowId(42));

        assert!(id1.is_same_entity(&id2));
    }

    #[test]
    fn test_is_same_entity_different_rowid() {
        let site = make_db_site(1);
        let id1 = EntityId::new(site.clone(), "posts_edit_context".to_string(), RowId(42));
        let id2 = EntityId::new(site, "posts_edit_context".to_string(), RowId(43));

        assert!(!id1.is_same_entity(&id2));
    }

    #[test]
    fn test_is_same_entity_different_table() {
        let site = make_db_site(1);
        let id1 = EntityId::new(site.clone(), "posts_edit_context".to_string(), RowId(42));
        let id2 = EntityId::new(site, "posts_view_context".to_string(), RowId(42));

        assert!(!id1.is_same_entity(&id2));
    }

    #[test]
    fn test_is_same_entity_different_site() {
        let site1 = make_db_site(1);
        let site2 = make_db_site(2);
        let id1 = EntityId::new(site1, "posts_edit_context".to_string(), RowId(42));
        let id2 = EntityId::new(site2, "posts_edit_context".to_string(), RowId(42));

        assert!(!id1.is_same_entity(&id2));
    }
}
