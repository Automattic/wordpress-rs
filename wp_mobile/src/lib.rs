// Re-export to ensure its bindings are generated
pub use wp_api;
pub use wp_mobile_cache;

#[uniffi::export]
fn wp_mobile_crate_works(input: String) -> String {
    format!("foo is {}", input)
}

uniffi::setup_scaffolding!();
