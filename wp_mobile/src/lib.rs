// Re-export to ensure its bindings are generated
pub use wp_api;
pub use wp_mobile_cache;

mod collection;
mod collection_error;
mod entity_error;
mod filters;
mod naive_collection;
mod service;

#[cfg(test)]
mod test_fixtures;

// Re-export types
pub use collection::{FetchError, FetchResult, PostCollection, PostCollectionWithEditContext};
pub use collection_error::CollectionError;
pub use entity_error::EntityError;
pub use filters::AnyPostFilter;
pub use naive_collection::NaiveCollection;

/// Macro to create UniFFI-compatible entity wrappers
///
/// This macro generates two types:
/// 1. Entity wrapper (e.g., EntityAnyPostWithEditContext) - handle to reload data
/// 2. FullEntity wrapper (e.g., FullEntityAnyPostWithEditContext) - data + EntityId
///
/// The FullEntity wrapper exposes both the EntityId and the data to UniFFI clients.
///
/// # Usage
/// ```ignore
/// wp_mobile_entity!(EntityAnyPostWithEditContext, wp_api::posts::AnyPostWithEditContext);
/// ```
/// This generates both `EntityAnyPostWithEditContext` and `FullEntityAnyPostWithEditContext`.
#[macro_export]
macro_rules! wp_mobile_entity {
    ($id_type:ident, $t_type:ty) => {
        paste::paste! {
            // FullEntity wrapper - pairs data with EntityId for UniFFI
            #[derive(uniffi::Record)]
            pub struct [<Full $id_type>] {
                pub entity_id: std::sync::Arc<wp_mobile_cache::entity::EntityId>,
                pub data: $t_type,
            }

            impl From<wp_mobile_cache::entity::FullEntity<$t_type>> for [<Full $id_type>] {
                fn from(value: wp_mobile_cache::entity::FullEntity<$t_type>) -> Self {
                    Self {
                        entity_id: value.entity_id,
                        data: value.data,
                    }
                }
            }

            // Entity wrapper - handle to reload data
            #[derive(uniffi::Object)]
            pub struct $id_type(pub wp_mobile_cache::entity::Entity<$t_type>);

            impl From<wp_mobile_cache::entity::Entity<$t_type>> for $id_type {
                fn from(value: wp_mobile_cache::entity::Entity<$t_type>) -> Self {
                    Self(value)
                }
            }

            #[uniffi::export]
            impl $id_type {
                /// Get the entity's ID
                pub fn id(&self) -> std::sync::Arc<wp_mobile_cache::entity::EntityId> {
                    std::sync::Arc::new(*self.0.id())
                }

                /// Load current data from cache/DB
                ///
                /// This is an expensive operation that reads from the database each time.
                /// Subsequent calls may return different results if the underlying data has changed.
                ///
                /// Returns:
                /// - Ok(Some(FullEntity)) if entity exists in cache (includes EntityId and data)
                /// - Ok(None) if entity not found in cache
                /// - Err(EntityError) if database error occurred
                pub fn load_data(
                    &self,
                ) -> Result<Option<[<Full $id_type>]>, $crate::entity_error::EntityError> {
                    self.0
                        .load_data()
                        .map(|opt| opt.map(|full_entity| full_entity.into()))
                        .map_err(|e| e.into())
                }

                /// Load current data from cache/DB (async version)
                ///
                /// This is an expensive operation that reads from the database each time.
                /// Subsequent calls may return different results if the underlying data has changed.
                ///
                /// Returns:
                /// - Ok(Some(FullEntity)) if entity exists in cache (includes EntityId and data)
                /// - Ok(None) if entity not found in cache
                /// - Err(EntityError) if database error occurred
                pub async fn load_data_async(
                    &self,
                ) -> Result<Option<[<Full $id_type>]>, $crate::entity_error::EntityError> {
                    self.0
                        .load_data()
                        .map(|opt| opt.map(|full_entity| full_entity.into()))
                        .map_err(|e| e.into())
                }

                /// Check if a database update is relevant to this entity
                ///
                /// This method allows platform-specific observable wrappers to determine
                /// whether they should notify observers about a database change.
                pub fn is_relevant_update(&self, hook: &wp_mobile_cache::UpdateHook) -> bool {
                    self.0.is_relevant_update(hook)
                }
            }
        }
    };
}

wp_mobile_entity!(
    EntityAnyPostWithEditContext,
    wp_api::posts::AnyPostWithEditContext
);

wp_mobile_naive_collection!(
    AnyPostWithEditContext,
    wp_api::posts::AnyPostWithEditContext
);

/// Macro to create UniFFI-compatible naive collection wrappers
///
/// This macro generates a wrapper type for `NaiveCollection<T>` that can be used
/// across language boundaries via UniFFI. The generated type includes methods for
/// loading data and checking update relevance.
///
/// The macro automatically generates the collection name by prepending "All" and
/// appending "Collection" to the entity name, and also auto-generates the full
/// entity type name by prepending "FullEntity" to the entity name.
///
/// # Parameters
/// - `$entity_name`: Base name for the entity (e.g., `AnyPostWithEditContext`)
/// - `$data_type`: The underlying data type (e.g., `wp_api::posts::AnyPostWithEditContext`)
///
/// # Usage
/// ```ignore
/// wp_mobile_naive_collection!(
///     AnyPostWithEditContext,
///     wp_api::posts::AnyPostWithEditContext
/// );
/// ```
///
/// This generates:
/// - `AllAnyPostWithEditContextCollection` - the collection wrapper type
/// - Uses `FullEntityAnyPostWithEditContext` - for the return type
#[macro_export]
macro_rules! wp_mobile_naive_collection {
    ($entity_name:ident, $data_type:ty) => {
        paste::paste! {
            #[derive(uniffi::Object)]
            pub struct [<All $entity_name Collection>](
                pub $crate::NaiveCollection<
                    wp_mobile_cache::entity::FullEntity<$data_type>,
                >,
            );

            impl From<
                    $crate::NaiveCollection<
                        wp_mobile_cache::entity::FullEntity<$data_type>,
                    >,
                > for [<All $entity_name Collection>]
            {
                fn from(
                    value: $crate::NaiveCollection<
                        wp_mobile_cache::entity::FullEntity<$data_type>,
                    >,
                ) -> Self {
                    Self(value)
                }
            }

            #[uniffi::export]
            impl [<All $entity_name Collection>] {
                /// Load all items in the collection from the database
                ///
                /// This is an expensive operation that reads from the database each time.
                /// It returns all items currently stored in the database that match the
                /// collection's criteria (site, context, etc.).
                ///
                /// Returns:
                /// - Ok(Vec<FullEntity>) - All items in the collection (may be empty)
                /// - Err(CollectionError) if a database error occurred
                pub fn load_data(
                    &self,
                ) -> Result<Vec<[<FullEntity $entity_name>]>, $crate::collection_error::CollectionError> {
                    self.0
                        .load_data()
                        .map(|full_entities| {
                            full_entities
                                .into_iter()
                                .map(|full_entity| full_entity.into())
                                .collect()
                        })
                        .map_err(|e| e.into())
                }

                /// Load all items in the collection from the database (async version)
                ///
                /// This is an expensive operation that reads from the database each time.
                /// It returns all items currently stored in the database that match the
                /// collection's criteria (site, context, etc.).
                ///
                /// Returns:
                /// - Ok(Vec<FullEntity>) - All items in the collection (may be empty)
                /// - Err(CollectionError) if a database error occurred
                pub async fn load_data_async(
                    &self,
                ) -> Result<Vec<[<FullEntity $entity_name>]>, $crate::collection_error::CollectionError> {
                    // For now, just call the sync version
                    // In the future, this could be optimized to run on a background thread
                    self.load_data()
                }

                /// Check if a database update is relevant to this collection
                ///
                /// Returns true if the updated table is one of the tables this collection monitors.
                /// This allows platform-specific observable wrappers to determine whether they should
                /// notify observers about a database change.
                pub fn is_relevant_update(&self, hook: &wp_mobile_cache::UpdateHook) -> bool {
                    self.0.is_relevant_update(hook)
                }
            }
        }
    };
}

#[uniffi::export]
fn wp_mobile_crate_works(input: String) -> String {
    format!("foo is {}", input)
}

uniffi::setup_scaffolding!();
