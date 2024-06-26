use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;
use std::sync::Arc;
use url::Url;

pub use login_client::WpLoginClient;
pub use login_error::FindApiUrlsError;

const KEY_APPLICATION_PASSWORDS: &str = "application-passwords";

mod login_client;
mod login_error;

#[derive(Debug, uniffi::Record)]
pub struct WpRestApiUrls {
    api_details: Arc<WpApiDetails>,
    api_root_url: String,
}

// After a successful login, the system will receive an OAuth callback with the login details
// embedded as query params. This function parses that URL and extracts the login details as an object.
#[uniffi::export]
pub fn extract_login_details_from_url(
    url: WpRestApiUrl,
) -> Option<WpApiApplicationPasswordDetails> {
    if let (Some(site_url), Some(user_login), Some(password)) =
        url.as_url()
            .query_pairs()
            .fold((None, None, None), |accum, (k, v)| {
                match k.to_string().as_str() {
                    "site_url" => (Some(v.to_string()), accum.1, accum.2),
                    "user_login" => (accum.0, Some(v.to_string()), accum.2),
                    "password" => (accum.0, accum.1, Some(v.to_string())),
                    _ => accum,
                }
            })
    {
        Some(WpApiApplicationPasswordDetails {
            site_url,
            user_login,
            password,
        })
    } else {
        None
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Object)]
pub struct WpApiDetails {
    pub name: String,
    pub description: String,
    pub url: String,
    pub home: String,
    pub gmt_offset: String,
    pub timezone_string: String,
    pub namespaces: Vec<String>,
    pub authentication: HashMap<String, WpRestApiAuthenticationScheme>,
    pub site_icon_url: String,
}

#[uniffi::export]
impl WpApiDetails {
    fn find_application_passwords_authentication_url(&self) -> Option<String> {
        self.authentication
            .get(KEY_APPLICATION_PASSWORDS)
            .map(|auth_scheme| auth_scheme.endpoints.authorization.clone())
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WpRestApiAuthenticationScheme {
    pub endpoints: WpRestApiAuthenticationEndpoint,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WpRestApiAuthenticationEndpoint {
    pub authorization: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WpApiApplicationPasswordDetails {
    pub site_url: String,
    pub user_login: String,
    pub password: String,
}

// A type that's guaranteed to represent a valid URL
//
// It is a programmer error to instantiate this object with an invalid URL
#[derive(Debug, uniffi::Record)]
pub struct WpRestApiUrl {
    pub string_value: String,
}

impl WpRestApiUrl {
    pub fn as_str(&self) -> &str {
        self.string_value.as_str()
    }

    pub fn as_url(&self) -> url::Url {
        Url::parse(self.string_value.as_str()).unwrap()
    }
}

impl From<Url> for WpRestApiUrl {
    fn from(url: url::Url) -> Self {
        WpRestApiUrl {
            string_value: url.into(),
        }
    }
}

impl From<WpRestApiUrl> for String {
    fn from(url: WpRestApiUrl) -> Self {
        url.string_value
    }
}
