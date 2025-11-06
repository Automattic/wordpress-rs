use crate::entity_error::EntityError;
use wp_mobile_cache::UpdateHook;

/// Lightweight handle to a single entity with state and metadata.
///
/// Entity is just an ID wrapper that reads data from global stores (cache DB and state store).
/// Multiple Entity instances with the same ID are considered equal and will read the same data.
pub struct Entity<T> {
    id: i64,
    read_data: Box<dyn Fn() -> Result<Option<T>, EntityError> + Send + Sync>,
    is_relevant_update: Box<dyn Fn(&UpdateHook) -> bool + Send + Sync>,
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
    pub(crate) fn new(
        id: i64,
        read_data: Box<dyn Fn() -> Result<Option<T>, EntityError> + Send + Sync>,
        is_relevant_update: Box<dyn Fn(&UpdateHook) -> bool + Send + Sync>,
    ) -> Self {
        Self {
            id,
            read_data,
            is_relevant_update,
        }
    }

    /// Get the entity's ID
    pub fn id(&self) -> i64 {
        self.id
    }

    /// Load current data from cache/DB
    ///
    /// This is an expensive operation that reads from the database each time.
    /// Subsequent calls may return different results if the underlying data has changed.
    ///
    /// Returns:
    /// - Ok(Some(T)) if entity exists in cache
    /// - Ok(None) if entity not found in cache
    /// - Err(EntityError) if database error occurred
    pub fn load_data(&self) -> Result<Option<T>, EntityError> {
        (self.read_data)()
    }

    /// Check if a database update is relevant to this entity
    ///
    /// This method allows platform-specific observable wrappers to determine
    /// whether they should notify observers about a database change.
    pub fn is_relevant_update(&self, hook: &UpdateHook) -> bool {
        (self.is_relevant_update)(hook)
    }

    // TODO: Add methods that will be implemented later:
    // pub fn state(&self) -> EntityState
    // pub fn last_fetched_at(&self) -> Option<String>
    // pub async fn refresh(&self) -> Result<(), EntityError>
}

// Note: PartialEq, Eq, and Clone are not implemented because the read_data closure
// cannot be cloned or compared. Entities are identified by ID conceptually,
// but Rust can't derive these traits with the closure field.

#[macro_export]
macro_rules! wp_mobile_entity {
    ($id_type:ident, $t_type:ty) => {
        #[derive(uniffi::Object)]
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

            /// Load current data from cache/DB
            ///
            /// This is an expensive operation that reads from the database each time.
            /// Subsequent calls may return different results if the underlying data has changed.
            ///
            /// Returns:
            /// - Ok(Some(T)) if entity exists in cache
            /// - Ok(None) if entity not found in cache
            /// - Err(EntityError) if database error occurred
            pub fn load_data(&self) -> Result<Option<$t_type>, $crate::entity_error::EntityError> {
                self.0.load_data()
            }

            /// Check if a database update is relevant to this entity
            ///
            /// This method allows platform-specific observable wrappers to determine
            /// whether they should notify observers about a database change.
            pub fn is_relevant_update(&self, hook: &wp_mobile_cache::UpdateHook) -> bool {
                self.0.is_relevant_update(hook)
            }
        }
    };
}
