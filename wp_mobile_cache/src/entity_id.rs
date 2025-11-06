/// Unique identifier for an entity stored in the cache database
///
/// Encapsulates the complete identity of a cached entity:
/// - Which database (site_id)
/// - Which table (table_name)
/// - Which row (rowid)
///
/// This type serves as an opaque handle that can be used to:
/// - Create observable entities without database lookups
/// - Compare entities for identity equality
/// - Filter database change notifications
///
/// EntityId is immutable once created and remains valid even if the
/// underlying database row is deleted (though queries may return None).
#[derive(uniffi::Object, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntityId {
    /// The site this entity belongs to
    site_id: i64,

    /// The table name where this entity is stored (e.g., "posts_edit_context")
    table_name: String,

    /// The database rowid (SQLite autoincrement primary key)
    rowid: i64,
}

#[uniffi::export]
impl EntityId {
    /// Check if two EntityIds refer to the same database entity
    ///
    /// Two EntityIds are considered the same if they have matching
    /// site_id, table_name, and rowid.
    pub fn is_same_entity(&self, other: &EntityId) -> bool {
        self.site_id == other.site_id
            && self.table_name == other.table_name
            && self.rowid == other.rowid
    }
}

impl EntityId {
    /// Create a new EntityId (internal only - not exposed via UniFFI)
    pub(crate) fn new(site_id: i64, table_name: String, rowid: i64) -> Self {
        Self {
            site_id,
            table_name,
            rowid,
        }
    }

    /// Get the rowid (internal only)
    pub(crate) fn rowid(&self) -> i64 {
        self.rowid
    }

    /// Get the table name (internal only)
    pub(crate) fn table_name(&self) -> &str {
        &self.table_name
    }

    /// Get the site_id (internal only)
    pub(crate) fn site_id(&self) -> i64 {
        self.site_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_same_entity_matching() {
        let id1 = EntityId::new(1, "posts_edit_context".to_string(), 42);
        let id2 = EntityId::new(1, "posts_edit_context".to_string(), 42);

        assert!(id1.is_same_entity(&id2));
    }

    #[test]
    fn test_is_same_entity_different_rowid() {
        let id1 = EntityId::new(1, "posts_edit_context".to_string(), 42);
        let id2 = EntityId::new(1, "posts_edit_context".to_string(), 43);

        assert!(!id1.is_same_entity(&id2));
    }

    #[test]
    fn test_is_same_entity_different_table() {
        let id1 = EntityId::new(1, "posts_edit_context".to_string(), 42);
        let id2 = EntityId::new(1, "posts_view_context".to_string(), 42);

        assert!(!id1.is_same_entity(&id2));
    }

    #[test]
    fn test_is_same_entity_different_site() {
        let id1 = EntityId::new(1, "posts_edit_context".to_string(), 42);
        let id2 = EntityId::new(2, "posts_edit_context".to_string(), 42);

        assert!(!id1.is_same_entity(&id2));
    }
}
