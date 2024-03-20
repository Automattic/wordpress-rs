use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;
use url::Url;

#[uniffi::export]
pub fn find_rest_endpoint(bytes: &[u8]) -> Option<WPRestAPIURL> {
    if let Ok(utf8_string) = str::from_utf8(bytes) {
        let document = Html::parse_document(utf8_string);
        let selector = Selector::parse("link").unwrap();

        // Go through each link and find the one that's `rel="https://api.w.org/"`, then extract it's
        // `href` attribute – that's the REST API root
        for element in document.select(&selector) {
            if element.value().attr("rel") == Some("https://api.w.org/") {
                if let Some(url_string) = element.value().attr("href") {
                    if let Ok(url) = Url::parse(url_string) {
                        return Some(url.into());
                    }
                }
            }
        }

        // TODO: This should throw some kind of error
        println!("HTML doc doesn't contain a WordPress.org API link");
        return None;
    }

    println!("Invalid utf-8");
    None
}

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

// A type that's guaranteed to represent a valid URL
//
// It is a programmer error to instantiate this object with an invalid URL
#[derive(Debug, uniffi::Record)]
pub struct WPRestAPIURL {
    pub string_value: String,
}

impl WPRestAPIURL {
    fn as_str(&self) -> &str {
        self.string_value.as_str()
    }

    fn as_url(&self) -> url::Url {
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
