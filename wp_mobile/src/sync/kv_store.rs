use std::collections::HashMap;
use std::hash::Hash;
use std::sync::RwLock;

use super::EntityMetadata;

/// Simple key-value store abstraction for metadata persistence.
///
/// This trait allows swapping between in-memory and persistent storage
/// implementations without changing the `MetadataCollection` logic.
///
/// # Type Parameter
/// - `Id`: The entity ID type (e.g., `PostId`, `MediaId`)
pub trait KvStore<Id>: Send + Sync
where
    Id: Clone + Eq + Hash + Send + Sync,
{
    /// Get metadata list for a key, if it exists.
    fn get(&self, key: &str) -> Option<Vec<EntityMetadata<Id>>>;

    /// Set (replace) metadata list for a key.
    fn set(&self, key: &str, value: Vec<EntityMetadata<Id>>);

    /// Append metadata to existing list for a key.
    /// If the key doesn't exist, creates a new list.
    fn append(&self, key: &str, value: Vec<EntityMetadata<Id>>);

    /// Remove metadata for a key.
    fn remove(&self, key: &str);

    /// Check if a key exists.
    fn contains(&self, key: &str) -> bool;
}

/// In-memory implementation of `KvStore`.
///
/// Useful for prototyping and testing. Data is lost when the process exits.
/// Can be swapped for a persistent implementation later.
pub struct InMemoryKvStore<Id>
where
    Id: Clone + Eq + Hash + Send + Sync,
{
    data: RwLock<HashMap<String, Vec<EntityMetadata<Id>>>>,
}

impl<Id> InMemoryKvStore<Id>
where
    Id: Clone + Eq + Hash + Send + Sync,
{
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }
}

impl<Id> Default for InMemoryKvStore<Id>
where
    Id: Clone + Eq + Hash + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Id> KvStore<Id> for InMemoryKvStore<Id>
where
    Id: Clone + Eq + Hash + Send + Sync,
{
    fn get(&self, key: &str) -> Option<Vec<EntityMetadata<Id>>> {
        self.data
            .read()
            .expect("RwLock poisoned")
            .get(key)
            .cloned()
    }

    fn set(&self, key: &str, value: Vec<EntityMetadata<Id>>) {
        self.data
            .write()
            .expect("RwLock poisoned")
            .insert(key.to_string(), value);
    }

    fn append(&self, key: &str, value: Vec<EntityMetadata<Id>>) {
        let mut data = self.data.write().expect("RwLock poisoned");
        data.entry(key.to_string())
            .or_insert_with(Vec::new)
            .extend(value);
    }

    fn remove(&self, key: &str) {
        self.data.write().expect("RwLock poisoned").remove(key);
    }

    fn contains(&self, key: &str) -> bool {
        self.data
            .read()
            .expect("RwLock poisoned")
            .contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wp_api::prelude::WpGmtDateTime;

    // Simple test ID type
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct TestId(i64);

    fn test_metadata(id: i64) -> EntityMetadata<TestId> {
        EntityMetadata::new(TestId(id), WpGmtDateTime::from_timestamp(1000 + id))
    }

    #[test]
    fn test_set_and_get() {
        let store = InMemoryKvStore::<TestId>::new();
        let metadata = vec![test_metadata(1), test_metadata(2)];

        store.set("posts:publish", metadata.clone());

        let result = store.get("posts:publish");
        assert_eq!(result, Some(metadata));
    }

    #[test]
    fn test_get_nonexistent_returns_none() {
        let store = InMemoryKvStore::<TestId>::new();

        assert_eq!(store.get("nonexistent"), None);
    }

    #[test]
    fn test_append_to_existing() {
        let store = InMemoryKvStore::<TestId>::new();

        store.set("posts:publish", vec![test_metadata(1)]);
        store.append("posts:publish", vec![test_metadata(2), test_metadata(3)]);

        let result = store.get("posts:publish").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].id, TestId(1));
        assert_eq!(result[1].id, TestId(2));
        assert_eq!(result[2].id, TestId(3));
    }

    #[test]
    fn test_append_to_nonexistent_creates_new() {
        let store = InMemoryKvStore::<TestId>::new();

        store.append("posts:publish", vec![test_metadata(1)]);

        let result = store.get("posts:publish").unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_remove() {
        let store = InMemoryKvStore::<TestId>::new();
        store.set("posts:publish", vec![test_metadata(1)]);

        store.remove("posts:publish");

        assert_eq!(store.get("posts:publish"), None);
    }

    #[test]
    fn test_contains() {
        let store = InMemoryKvStore::<TestId>::new();

        assert!(!store.contains("posts:publish"));

        store.set("posts:publish", vec![test_metadata(1)]);

        assert!(store.contains("posts:publish"));
    }
}
