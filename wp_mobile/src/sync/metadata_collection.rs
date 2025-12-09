use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

use wp_api::prelude::WpGmtDateTime;
use wp_mobile_cache::entity::FullEntity;

use super::{EntityMetadata, KvStore, ListItem};

/// Collection that uses metadata-first fetching strategy.
///
/// This collection type:
/// 1. Uses lightweight metadata (id + modified_gmt) to define list structure
/// 2. Shows cached entities immediately, with loading placeholders for missing items
/// 3. Tracks which entities are missing or stale for selective fetching
///
/// # Type Parameters
/// - `T`: The full entity type (e.g., `AnyPostWithEditContext`)
/// - `Id`: The ID type (e.g., `PostId`)
///
/// # Usage Flow
/// 1. Call `load_from_kv_store()` to get initial list from persisted metadata
/// 2. Call `set_metadata()` after fetching fresh metadata from network
/// 3. Call `load_data()` to build list with loaded/loading/failed states
/// 4. Call `get_missing_ids()` or `get_stale_ids()` to determine what to fetch
/// 5. After fetching, call `load_data()` again to get updated list
pub struct MetadataCollection<T, Id>
where
    Id: Clone + Eq + Hash + Send + Sync,
{
    /// Key for KV store lookup
    kv_key: String,

    /// KV store for metadata persistence
    kv_store: Arc<dyn KvStore<Id>>,

    /// Current metadata defining the list structure
    /// None means no metadata has been loaded/fetched yet
    metadata: Option<Vec<EntityMetadata<Id>>>,

    /// Closure to load an entity from cache by ID
    /// Returns None if entity is not in cache
    load_entity_by_id:
        Box<dyn Fn(&Id) -> Result<Option<FullEntity<T>>, LoadError> + Send + Sync>,

    /// Closure to get modified_gmt for a cached entity by ID
    /// Used to determine staleness without loading full entity
    get_cached_modified_gmt:
        Box<dyn Fn(&Id) -> Result<Option<WpGmtDateTime>, LoadError> + Send + Sync>,
}

/// Error type for load operations
#[derive(Debug, Clone)]
pub struct LoadError {
    pub message: String,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LoadError {}

impl<T, Id> MetadataCollection<T, Id>
where
    Id: Clone + Eq + Hash + Send + Sync,
{
    /// Create a new metadata collection.
    ///
    /// # Arguments
    /// * `kv_key` - Key for KV store persistence
    /// * `kv_store` - KV store for metadata persistence
    /// * `load_entity_by_id` - Closure to load full entity from cache
    /// * `get_cached_modified_gmt` - Closure to get cached entity's modified_gmt
    pub fn new(
        kv_key: String,
        kv_store: Arc<dyn KvStore<Id>>,
        load_entity_by_id: Box<
            dyn Fn(&Id) -> Result<Option<FullEntity<T>>, LoadError> + Send + Sync,
        >,
        get_cached_modified_gmt: Box<
            dyn Fn(&Id) -> Result<Option<WpGmtDateTime>, LoadError> + Send + Sync,
        >,
    ) -> Self {
        Self {
            kv_key,
            kv_store,
            metadata: None,
            load_entity_by_id,
            get_cached_modified_gmt,
        }
    }

    /// Load metadata from KV store.
    ///
    /// Call this on initial load to restore persisted list structure.
    /// Returns true if metadata was found in KV store.
    pub fn load_from_kv_store(&mut self) -> bool {
        if let Some(metadata) = self.kv_store.get(&self.kv_key) {
            self.metadata = Some(metadata);
            true
        } else {
            false
        }
    }

    /// Set metadata from a fresh network fetch.
    ///
    /// # Arguments
    /// * `metadata` - Fresh metadata from network
    /// * `is_first_page` - If true, replaces existing metadata; if false, appends
    pub fn set_metadata(&mut self, metadata: Vec<EntityMetadata<Id>>, is_first_page: bool) {
        if is_first_page {
            self.kv_store.set(&self.kv_key, metadata.clone());
            self.metadata = Some(metadata);
        } else {
            self.kv_store.append(&self.kv_key, metadata.clone());
            if let Some(ref mut existing) = self.metadata {
                existing.extend(metadata);
            } else {
                self.metadata = Some(metadata);
            }
        }
    }

    /// Check if metadata has been loaded.
    pub fn has_metadata(&self) -> bool {
        self.metadata.is_some()
    }

    /// Get the current metadata, if any.
    pub fn metadata(&self) -> Option<&[EntityMetadata<Id>]> {
        self.metadata.as_deref()
    }

    /// Clear metadata from memory and KV store.
    pub fn clear(&mut self) {
        self.metadata = None;
        self.kv_store.remove(&self.kv_key);
    }

    /// Build the list with current load states.
    ///
    /// Returns a list of `ListItem` where each item is either:
    /// - `Loaded`: Full entity from cache
    /// - `Loading`: Placeholder for entity not in cache
    ///
    /// Note: This doesn't set `Failed` state - that's managed externally
    /// based on fetch results.
    pub fn load_data(&self) -> Result<Vec<ListItem<T, Id>>, LoadError> {
        let Some(metadata) = &self.metadata else {
            return Ok(Vec::new());
        };

        metadata
            .iter()
            .map(|meta| {
                match (self.load_entity_by_id)(&meta.id)? {
                    Some(entity) => Ok(ListItem::Loaded(entity)),
                    None => Ok(ListItem::Loading(meta.clone())),
                }
            })
            .collect()
    }

    /// Get IDs of entities that are not in the cache.
    pub fn get_missing_ids(&self) -> Result<Vec<Id>, LoadError> {
        let Some(metadata) = &self.metadata else {
            return Ok(Vec::new());
        };

        let mut missing = Vec::new();
        for meta in metadata {
            let cached_modified = (self.get_cached_modified_gmt)(&meta.id)?;
            if cached_modified.is_none() {
                missing.push(meta.id.clone());
            }
        }
        Ok(missing)
    }

    /// Get IDs of entities that are in cache but have different modified_gmt.
    pub fn get_stale_ids(&self) -> Result<Vec<Id>, LoadError> {
        let Some(metadata) = &self.metadata else {
            return Ok(Vec::new());
        };

        let mut stale = Vec::new();
        for meta in metadata {
            if let Some(cached_modified) = (self.get_cached_modified_gmt)(&meta.id)? {
                if cached_modified != meta.modified_gmt {
                    stale.push(meta.id.clone());
                }
            }
        }
        Ok(stale)
    }

    /// Get IDs of entities that need fetching (missing or stale).
    pub fn get_ids_needing_fetch(&self) -> Result<Vec<Id>, LoadError> {
        let Some(metadata) = &self.metadata else {
            return Ok(Vec::new());
        };

        let mut needs_fetch = Vec::new();
        for meta in metadata {
            let cached_modified = (self.get_cached_modified_gmt)(&meta.id)?;
            match cached_modified {
                None => needs_fetch.push(meta.id.clone()),
                Some(cached) if cached != meta.modified_gmt => {
                    needs_fetch.push(meta.id.clone())
                }
                _ => {}
            }
        }
        Ok(needs_fetch)
    }

    /// Build a map of ID -> ListItem for efficient lookups.
    ///
    /// Useful when you need to update specific items in the list.
    pub fn load_data_as_map(&self) -> Result<HashMap<Id, ListItem<T, Id>>, LoadError> {
        let items = self.load_data()?;
        let Some(metadata) = &self.metadata else {
            return Ok(HashMap::new());
        };

        Ok(metadata
            .iter()
            .zip(items)
            .map(|(meta, item)| (meta.id.clone(), item))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::InMemoryKvStore;
    use std::sync::Arc;
    use wp_mobile_cache::{DbTable, RowId, db_types::db_site::DbSite};

    // Simple test types
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct TestId(i64);

    #[derive(Debug, Clone)]
    struct TestEntity {
        id: TestId,
        title: String,
    }

    fn test_metadata(id: i64) -> EntityMetadata<TestId> {
        EntityMetadata::new(TestId(id), WpGmtDateTime::from_timestamp(1000 + id))
    }

    fn test_db_site() -> DbSite {
        DbSite {
            row_id: RowId(1),
            site_type: wp_mobile_cache::db_types::db_site::DbSiteType::SelfHosted,
            mapped_site_id: RowId(1),
        }
    }

    fn create_test_collection(
        cached_modified_gmts: HashMap<TestId, WpGmtDateTime>,
    ) -> MetadataCollection<TestEntity, TestId> {
        let kv_store = Arc::new(InMemoryKvStore::<TestId>::new());
        let db_site = test_db_site();
        let cached = Arc::new(cached_modified_gmts);
        let cached_clone = cached.clone();

        MetadataCollection::new(
            "test_key".to_string(),
            kv_store,
            // For load_entity_by_id, return a FullEntity if cached
            Box::new(move |id| {
                Ok(cached.get(id).map(|_| {
                    let entity_id = Arc::new(wp_mobile_cache::entity::EntityId {
                        db_site,
                        table: DbTable::PostsEditContext,
                        rowid: RowId(id.0),
                    });
                    FullEntity::new(
                        entity_id,
                        TestEntity {
                            id: *id,
                            title: format!("Test {}", id.0),
                        },
                    )
                }))
            }),
            // For get_cached_modified_gmt, return the cached timestamp
            Box::new(move |id| Ok(cached_clone.get(id).copied())),
        )
    }

    #[test]
    fn test_empty_collection_returns_empty_list() {
        let collection = create_test_collection(HashMap::new());
        let items = collection.load_data().unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn test_set_metadata_persists_to_kv_store() {
        let kv_store = Arc::new(InMemoryKvStore::<TestId>::new());
        let kv_store_clone = kv_store.clone();

        let mut collection = MetadataCollection::<TestEntity, TestId>::new(
            "test_key".to_string(),
            kv_store,
            Box::new(|_| Ok(None)),
            Box::new(|_| Ok(None)),
        );

        let metadata = vec![test_metadata(1), test_metadata(2)];
        collection.set_metadata(metadata.clone(), true);

        // Verify KV store has the metadata
        let stored = kv_store_clone.get("test_key").unwrap();
        assert_eq!(stored.len(), 2);
    }

    #[test]
    fn test_load_data_returns_loading_for_missing() {
        let mut collection = create_test_collection(HashMap::new());

        collection.set_metadata(vec![test_metadata(1), test_metadata(2)], true);

        let items = collection.load_data().unwrap();
        assert_eq!(items.len(), 2);
        assert!(items[0].is_loading());
        assert!(items[1].is_loading());
    }

    #[test]
    fn test_load_data_returns_loaded_for_cached() {
        let mut cached = HashMap::new();
        cached.insert(TestId(1), WpGmtDateTime::from_timestamp(1001));

        let mut collection = create_test_collection(cached);
        collection.set_metadata(vec![test_metadata(1), test_metadata(2)], true);

        let items = collection.load_data().unwrap();
        assert_eq!(items.len(), 2);
        assert!(items[0].is_loaded());
        assert!(items[1].is_loading());
    }

    #[test]
    fn test_get_missing_ids() {
        let mut cached = HashMap::new();
        cached.insert(TestId(1), WpGmtDateTime::from_timestamp(1001));

        let mut collection = create_test_collection(cached);
        collection.set_metadata(
            vec![test_metadata(1), test_metadata(2), test_metadata(3)],
            true,
        );

        let missing = collection.get_missing_ids().unwrap();
        assert_eq!(missing, vec![TestId(2), TestId(3)]);
    }

    #[test]
    fn test_get_stale_ids() {
        let mut cached = HashMap::new();
        // Post 1: cached with matching timestamp
        cached.insert(TestId(1), WpGmtDateTime::from_timestamp(1001)); // Matches test_metadata(1)
        // Post 2: cached with different timestamp (stale)
        cached.insert(TestId(2), WpGmtDateTime::from_timestamp(9999)); // Different from test_metadata(2)

        let mut collection = create_test_collection(cached);
        collection.set_metadata(vec![test_metadata(1), test_metadata(2)], true);

        let stale = collection.get_stale_ids().unwrap();
        assert_eq!(stale, vec![TestId(2)]);
    }

    #[test]
    fn test_append_metadata() {
        let mut collection = create_test_collection(HashMap::new());

        // First page
        collection.set_metadata(vec![test_metadata(1), test_metadata(2)], true);
        assert_eq!(collection.metadata().unwrap().len(), 2);

        // Second page (append)
        collection.set_metadata(vec![test_metadata(3), test_metadata(4)], false);
        assert_eq!(collection.metadata().unwrap().len(), 4);
    }

    #[test]
    fn test_load_from_kv_store() {
        let kv_store = Arc::new(InMemoryKvStore::<TestId>::new());
        kv_store.set("test_key", vec![test_metadata(1), test_metadata(2)]);

        let mut collection = MetadataCollection::<TestEntity, TestId>::new(
            "test_key".to_string(),
            kv_store,
            Box::new(|_| Ok(None)),
            Box::new(|_| Ok(None)),
        );

        assert!(!collection.has_metadata());
        let loaded = collection.load_from_kv_store();
        assert!(loaded);
        assert!(collection.has_metadata());
        assert_eq!(collection.metadata().unwrap().len(), 2);
    }

    #[test]
    fn test_clear() {
        let kv_store = Arc::new(InMemoryKvStore::<TestId>::new());
        let kv_store_clone = kv_store.clone();

        let mut collection = MetadataCollection::<TestEntity, TestId>::new(
            "test_key".to_string(),
            kv_store,
            Box::new(|_| Ok(None)),
            Box::new(|_| Ok(None)),
        );

        collection.set_metadata(vec![test_metadata(1)], true);
        assert!(collection.has_metadata());
        assert!(kv_store_clone.contains("test_key"));

        collection.clear();
        assert!(!collection.has_metadata());
        assert!(!kv_store_clone.contains("test_key"));
    }
}
