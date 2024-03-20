use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;
use crate::url::*;

// After a successful login, the system will receive an OAuth callback with the login details
// embedded as query params. This function parses that URL and extracts the login details as an object.
#[uniffi::export]
pub fn extract_login_details_from_url(
    url: WPRestAPIURL,
) -> Option<WPAPIApplicationPasswordDetails> {
    let mut map = HashMap::new();

    for pair in url.as_url().query_pairs() {
        map.insert(pair.0.to_string(), pair.1.to_string());
    }

    println!("{:?}", map);

    if !map.contains_key("site_url")
        || !map.contains_key("user_login")
        || !map.contains_key("password")
    {
        return None;
    }

    Some(WPAPIApplicationPasswordDetails {
        site_url: map["site_url"].clone(),
        user_login: map["user_login"].clone(),
        password: map["password"].clone(),
    })
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
