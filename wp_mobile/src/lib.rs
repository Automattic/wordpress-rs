// Re-export to ensure bindings are generated
pub use wp_api;
pub use wp_mobile_cache;

mod cache_key;
pub mod collection;
pub mod entity;
pub mod filters;
pub mod persistence;
pub mod service;
pub mod sync;

#[cfg(test)]
mod testing;

// Generate concrete types after all modules are loaded
// This ensures entity types exist before collection macros reference them

wp_mobile_entity!(
    EntityAnyPostWithEditContext,
    wp_api::posts::AnyPostWithEditContext
);

wp_mobile_entity!(
    EntityPostTypeDetailsWithEditContext,
    wp_api::post_types::PostTypeDetailsWithEditContext
);

wp_mobile_post_collection!(
    PostCollectionWithEditContext,
    AnyPostWithEditContext,
    wp_api::posts::AnyPostWithEditContext
);

wp_mobile_stateless_collection!(
    AnyPostWithEditContext,
    wp_api::posts::AnyPostWithEditContext
);

uniffi::setup_scaffolding!();
