use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;
use url::Url;

// After a successful login, the system will receive an OAuth callback with the login details
// embedded as query params. This function parses that URL and extracts the login details as an object.
#[uniffi::export]
pub fn extract_login_details_from_url(
    url: WPRestAPIURL,
) -> Option<WPAPIApplicationPasswordDetails> {
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
        Some(WPAPIApplicationPasswordDetails {
            site_url,
            user_login,
            password,
        })
    } else {
        None
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPAPIDetails {
    pub name: String,
    pub description: String,
    pub url: String,
    pub home: String,
    pub gmt_offset: String,
    pub timezone_string: String,
    pub namespaces: Vec<String>,
    pub authentication: HashMap<String, WPRestAPIAuthenticationScheme>,
    pub site_icon_url: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPRestAPIAuthenticationScheme {
    pub endpoints: WPRestApiAuthenticationEndpoint,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPRestApiAuthenticationEndpoint {
    pub authorization: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPAPIApplicationPasswordDetails {
    pub site_url: String,
    pub user_login: String,
    pub password: String,
}

// A type that's guaranteed to represent a valid URL
//
// It is a programmer error to instantiate this object with an invalid URL
#[derive(Debug, uniffi::Record)]
pub struct WPRestAPIURL {
    pub string_value: String,
}

impl WPRestAPIURL {
    pub fn as_str(&self) -> &str {
        self.string_value.as_str()
    }

    pub fn as_url(&self) -> url::Url {
        Url::parse(self.string_value.as_str()).unwrap()
    }
}

impl From<Url> for WPRestAPIURL {
    fn from(url: url::Url) -> Self {
        WPRestAPIURL {
            string_value: url.into(),
        }
    }
}

impl From<WPRestAPIURL> for String {
    fn from(url: WPRestAPIURL) -> Self {
        url.string_value
    }
}
