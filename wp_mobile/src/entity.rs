use crate::entity_error::EntityError;

/// Lightweight handle to a single entity with observable state and metadata.
///
/// Entity is just an ID wrapper that reads data from global stores (cache DB and state store).
/// Multiple Entity instances with the same ID are considered equal and will read the same data.
pub struct Entity<T> {
    id: i64,
    read_data: Box<dyn Fn() -> Result<Option<T>, EntityError> + Send + Sync>,
    // TODO: Add trait reference for state_reader
    // state_reader: Arc<dyn StateReader>,
}

impl<T> Entity<T> {
    /// Create a new entity handle for the given ID
    ///
    /// The read_data closure is provided by the service layer and encapsulates
    /// the logic for reading entity data from the cache/DB.
    pub(crate) fn new(
        id: i64,
        read_data: Box<dyn Fn() -> Result<Option<T>, EntityError> + Send + Sync>,
    ) -> Self {
        Self {
            id,
            read_data,
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

    // TODO: Add methods that will be implemented later:
    // pub fn state(&self) -> EntityState
    // pub fn last_fetched_at(&self) -> Option<String>
    // pub async fn refresh(&self) -> Result<(), EntityError>
    // pub fn set_observer(&self, observer: Arc<dyn EntityObserver<T>>)
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
        }
    };
}
