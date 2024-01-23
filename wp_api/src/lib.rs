#![allow(dead_code, unused_variables)]

pub use api_error::*;
pub use pages::*;
pub use posts::*;

pub mod api_error;
pub mod pages;
pub mod posts;

#[derive(Debug, Clone)]
// TODO: This will probably become an `enum` where we support multiple authentication types.
pub struct WPAuthentication {
    pub auth_token: String,
}

uniffi::include_scaffolding!("wp_api");
