use std::sync::Arc;
use wp_mobile_cache::db_types::db_site::DbSite;

/// Lightweight handle to a single entity with observable state and metadata.
///
/// Entity is just an ID wrapper that reads data from global stores (cache DB and state store).
/// Multiple Entity instances with the same ID are considered equal and will read the same data.
#[derive(Debug)]
pub struct Entity<T> {
    id: i64,
    db_site: Arc<DbSite>,

    // TODO: Add trait references for state_reader and data_reader
    // state_reader: Arc<dyn StateReader>,
    // data_reader: Arc<dyn DataReader<T>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Entity<T> {
    /// Create a new entity handle for the given ID
    pub(crate) fn new(id: i64, db_site: Arc<DbSite>) -> Self {
        Self {
            id,
            db_site,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get the entity's ID
    pub fn id(&self) -> i64 {
        self.id
    }

    // TODO: Add methods that will be implemented later:
    // pub fn data(&self) -> Option<T>
    // pub fn state(&self) -> EntityState
    // pub fn last_fetched_at(&self) -> Option<String>
    // pub async fn refresh(&self) -> Result<(), EntityError>
    // pub fn set_observer(&self, observer: Arc<dyn EntityObserver<T>>)
}

impl<T> PartialEq for Entity<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.db_site == other.db_site
    }
}

impl<T> Eq for Entity<T> {}

impl<T> Clone for Entity<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            db_site: self.db_site.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

#[macro_export]
macro_rules! wp_mobile_entity {
    ($id_type:ident, $t_type:ty) => {
        #[derive(Debug, uniffi::Object)]
        pub struct $id_type(pub $crate::entity::Entity<$t_type>);

        impl From<crate::entity::Entity<$t_type>> for $id_type {
            fn from(value: crate::entity::Entity<$t_type>) -> Self {
                Self(value)
            }
        }

        #[uniffi::export]
        impl $id_type {
            /// Get the entity's ID
            pub fn id(&self) -> i64 {
                self.0.id()
            }

            /// Get current data (reads from cache/DB)
            /// Returns None for now - will be implemented later
            pub fn data(&self) -> Option<$t_type> {
                // TODO: Implement data reading from cache
                None
            }
        }
    };
}
