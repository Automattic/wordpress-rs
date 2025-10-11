// Re-export wp_api to ensure its bindings are generated
pub use wp_api;

#[uniffi::export]
fn wp_mobile_crate_works(input: String) -> String {
    format!("foo is {}", input)
}

uniffi::setup_scaffolding!();
