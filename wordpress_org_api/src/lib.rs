mod de;

pub mod plugin_directory;

mod client;
pub use client::*;

#[cfg(feature = "reqwest")]
pub mod reqwest;

pub type Result<T> = std::result::Result<T, self::WordPressOrgApiClientError>;

uniffi::setup_scaffolding!();
