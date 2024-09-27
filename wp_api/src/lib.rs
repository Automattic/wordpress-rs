#![allow(dead_code, unused_variables)]

pub use api_client::{WpApiClient, WpApiRequestBuilder};
pub use api_error::{RequestExecutionError, WpApiError, WpErrorCode};
pub use parsed_url::{ParseUrlError, ParsedUrl};
use plugins::*;
use url_query::AsQueryValue;
use users::*;
pub use uuid::{WpUuid, WpUuidParseError};

mod api_client; // re-exported relevant types
mod api_error; // re-exported relevant types
mod parsed_url; // re-exported relevant types
mod uuid; // re-exported relevant types

pub mod application_passwords;
pub mod login;
pub mod plugins;
pub mod post_types;
pub mod posts;
pub mod request;
pub mod site_settings;
pub mod url_query;
pub mod users;
pub mod wp_site_health_tests;

#[cfg(test)]
mod unit_test_common;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WpContext {
    Edit,
    Embed,
    #[default]
    View,
}

impl WpContext {
    fn as_str(&self) -> &str {
        match self {
            Self::Edit => "edit",
            Self::Embed => "embed",
            Self::View => "view",
        }
    }
}

/// WordPress site user account which is used to login from wp-login.php.
#[derive(Debug, Clone, uniffi::Record)]
pub struct WpLoginCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum WpAuthentication {
    AuthorizationHeader { token: String },
    UserAccount { login: WpLoginCredentials },
    None,
}

impl WpAuthentication {
    pub fn from_username_and_password(username: String, password: String) -> Self {
        use base64::prelude::*;
        WpAuthentication::AuthorizationHeader {
            token: BASE64_STANDARD.encode(format!("{}:{}", username, password)),
        }
    }
}

#[uniffi::export]
fn wp_authentication_from_username_and_password(
    username: String,
    password: String,
) -> WpAuthentication {
    WpAuthentication::from_username_and_password(username, password)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WpApiParamOrder {
    #[default]
    Asc,
    Desc,
}

impl_as_query_value_from_as_str!(WpApiParamOrder);

impl WpApiParamOrder {
    fn as_str(&self) -> &str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

trait SparseField {
    fn as_str(&self) -> &str;
}

#[macro_export]
macro_rules! generate {
    ($type_name:ident) => {
        $type_name::default()
    };
    ($type_name:ident, $(($f:ident, $v:expr)), *) => {{
        let mut obj = $type_name::default();
        $(obj.$f = $v;)*
        obj
    }};
}

uniffi::setup_scaffolding!();
