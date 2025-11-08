use crate::{RowId, SqliteDbError, UpdateHook, db_types::db_site::DbSite};
use std::sync::Arc;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Hashable key derived from an EntityId
///
/// This type can be used as a key in HashMap/HashSet on platforms like Kotlin
/// where EntityId (as a uniffi::Object) cannot be used directly as a HashMap key.
///
/// EntityKey is derived from EntityId and uniquely identifies an entity through
/// three opaque parts that together form a composite key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Record)]
pub struct EntityKey {
    /// First part of the composite key
    pub part1: i64,
    /// Second part of the composite key
    pub part2: u32,
    /// Third part of the composite key
    pub part3: u64,
}

/// Helper function to create a stable hash of a table name
fn hash_table_name(table_name: &str) -> u32 {
    let mut hasher = DefaultHasher::new();
    table_name.hash(&mut hasher);
    hasher.finish() as u32
}

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Object)]
pub struct EntityId {
    /// The site this entity belongs to
    pub db_site: DbSite,

    /// The table name where this entity is stored (e.g., "posts_edit_context")
    ///
    /// Uses a static string reference to avoid heap allocation and keep EntityId small.
    pub table_name: &'static str,

    /// The database rowid (SQLite autoincrement primary key)
    pub rowid: RowId,
}

impl EntityId {
    /// Create a new EntityId (internal only - not exposed via UniFFI)
    pub(crate) fn new(db_site: DbSite, table_name: &'static str, rowid: RowId) -> Self {
        Self {
            db_site,
            table_name,
            rowid,
        }
    }

    /// Validate that this EntityId's table name matches the expected table.
    ///
    /// Returns an error if the table names don't match.
    pub fn validate_table_name(&self, expected: &'static str) -> Result<(), SqliteDbError> {
        if self.table_name != expected {
            return Err(SqliteDbError::TableNameMismatch {
                expected: expected.to_string(),
                actual: self.table_name.to_string(),
            });
        }

        Ok(())
    }
}

#[uniffi::export]
impl EntityId {
    /// Convert this EntityId to an EntityKey that can be used as a HashMap key
    ///
    /// Returns an EntityKey that uniquely identifies this entity and can be used
    /// on platforms where EntityId cannot be used directly as a HashMap key.
    pub fn to_key(&self) -> EntityKey {
        EntityKey {
            part1: self.db_site.row_id.0 as i64,
            part2: hash_table_name(self.table_name),
            part3: self.rowid.0,
        }
    }
}

/// Wrapper that pairs cached data with its database identity
///
/// When fetching data from the cache, we return both the data and
/// an EntityId that can be used to:
/// - Create observable entities without additional database queries
/// - Identify this specific entity in update notifications
/// - Compare entities for identity equality
///
/// This type is generic over the data type T
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullEntity<T> {
    /// The database identity of this entity
    pub entity_id: Arc<EntityId>,

    /// The cached data
    pub data: T,
}

impl<T> FullEntity<T> {
    /// Create a new FullEntity pairing data with its identity
    pub fn new(entity_id: Arc<EntityId>, data: T) -> Self {
        Self { entity_id, data }
    }
}

/// Lightweight handle to a single entity with state and metadata.
///
/// Entity is just an ID wrapper that reads data from global stores (cache DB and state store).
/// Multiple Entity instances with the same ID are considered equal and will read the same data.
pub struct Entity<T> {
    entity_id: EntityId,
    read_data: Box<dyn Fn() -> Result<Option<FullEntity<T>>, SqliteDbError> + Send + Sync>,
    // TODO: Add trait reference for state_reader
    // state_reader: Arc<dyn StateReader>,
}

impl<T> Entity<T> {
    /// Create a new entity handle for the given ID
    ///
    /// The read_data closure is provided by the service layer and encapsulates
    /// the logic for reading entity data from the cache/DB.
    ///
    /// The is_relevant_update closure is provided by the service layer and determines
    /// whether a database update is relevant to this entity.
    pub fn new(
        entity_id: EntityId,
        read_data: Box<dyn Fn() -> Result<Option<FullEntity<T>>, SqliteDbError> + Send + Sync>,
    ) -> Self {
        Self {
            entity_id,
            read_data,
        }
    }

    /// Get the entity's ID
    pub fn id(&self) -> &EntityId {
        &self.entity_id
    }

    /// Load current data from cache/DB
    ///
    /// This is an expensive operation that reads from the database each time.
    /// Subsequent calls may return different results if the underlying data has changed.
    ///
    /// Returns:
    /// - Ok(Some(FullEntity<T>)) if entity exists in cache (includes EntityId and data)
    /// - Ok(None) if entity not found in cache
    /// - Err(SqliteDbError) if database error occurred
    pub fn load_data(&self) -> Result<Option<FullEntity<T>>, SqliteDbError> {
        (self.read_data)()
    }

    /// Check if a database update is relevant to this entity
    ///
    /// This method allows platform-specific observable wrappers to determine
    /// whether they should notify observers about a database change.
    pub fn is_relevant_update(&self, hook: &UpdateHook) -> bool {
        self.entity_id.table_name == hook.table_name && self.entity_id.rowid == hook.row_id.into()
    }

    // TODO: Add methods that will be implemented later:
    // pub fn state(&self) -> EntityState
    // pub fn last_fetched_at(&self) -> Option<String>
    // pub async fn refresh(&self) -> Result<(), SqliteDbError>
}

// Note: PartialEq, Eq, and Clone are not implemented because the read_data closure
// cannot be cloned or compared. Entities are identified by ID conceptually,
// but Rust can't derive these traits with the closure field.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_types::db_site::DbSiteType;

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
        let id1 = EntityId::new(site, "posts_edit_context", RowId(42));
        let id2 = EntityId::new(site, "posts_edit_context", RowId(42));

        assert_eq!(id1, id2);
    }

    #[test]
    fn test_not_same_entity_different_rowid() {
        let site = make_db_site(1);
        let id1 = EntityId::new(site, "posts_edit_context", RowId(42));
        let id2 = EntityId::new(site, "posts_edit_context", RowId(43));

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_not_same_entity_different_table() {
        let site = make_db_site(1);
        let id1 = EntityId::new(site, "posts_edit_context", RowId(42));
        let id2 = EntityId::new(site, "posts_view_context", RowId(42));

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_not_is_same_entity_different_site() {
        let site1 = make_db_site(1);
        let site2 = make_db_site(2);
        let id1 = EntityId::new(site1, "posts_edit_context", RowId(42));
        let id2 = EntityId::new(site2, "posts_edit_context", RowId(42));

        assert_ne!(id1, id2);
    }
}
