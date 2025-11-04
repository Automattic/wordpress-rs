// Re-export to ensure its bindings are generated
pub use wp_api;
pub use wp_mobile_cache;

mod entity;
mod entity_error;
mod service;

// Re-export error types
pub use entity_error::EntityError;

wp_mobile_entity!(
    EntityAnyPostWithEditContext,
    wp_api::posts::AnyPostWithEditContext
);

#[uniffi::export]
fn wp_mobile_crate_works(input: String) -> String {
    format!("foo is {}", input)
}

uniffi::setup_scaffolding!();
