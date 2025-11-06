// Re-export to ensure its bindings are generated
pub use wp_api;
pub use wp_mobile_cache;

mod entity_error;
mod service;

#[cfg(test)]
mod test_fixtures;

// Re-export error types
pub use entity_error::EntityError;

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
                pub fn id(&self) -> i64 {
                    self.0.id()
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

#[uniffi::export]
fn wp_mobile_crate_works(input: String) -> String {
    format!("foo is {}", input)
}

uniffi::setup_scaffolding!();
