use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;
use std::sync::Arc;
use url::Url;

use crate::parser::{ParseSiteUrlError, ParsedSiteUrl};
use crate::request::endpoint::WpEndpointUrl;
use crate::request::{RequestExecutor, RequestMethod, WpNetworkRequest, WpNetworkResponse};
use crate::{RequestExecutionError, WpApiError};

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum FindApiUrlsError {
    #[error("Not yet implemented application password authorization endpoint not found")]
    ApplicationPasswordEndpointNotFound,
    #[error("Not yet implemented capabilities response error")]
    CapabilitiesResponseError,
    #[error(transparent)]
    ParseSiteUrlError(#[from] ParseSiteUrlError),
    #[error("Link Header not found in response")]
    LinkHeaderNotFound,
    #[error("Not yet implemented")]
    NotYetImplemented,
}

impl From<RequestExecutionError> for FindApiUrlsError {
    fn from(value: RequestExecutionError) -> Self {
        Self::NotYetImplemented
    }
}

#[derive(uniffi::Record)]
pub struct WpRestApiRootUrl {}

#[derive(uniffi::Record)]
pub struct WpRestApiUrls {
    api_root_url: String,
    application_password_authentication_url: String,
}

#[uniffi::export]
pub async fn find_api_urls(
    site_url: String,
    request_executor: Arc<dyn RequestExecutor>,
) -> Result<WpRestApiUrls, FindApiUrlsError> {
    // 1. Parse the URL to standardize its format (so "example.com" would become "https://example.com")
    let parsed_site_url = ParsedSiteUrl::parse_str(site_url)?;

    // 2. Fetches the site's homepage with a HEAD request
    let api_root_request = WpNetworkRequest {
        method: RequestMethod::HEAD,
        url: WpEndpointUrl(parsed_site_url.site_url.to_string()),
        header_map: HashMap::new(),
        body: None,
    };
    let api_root_response = request_executor.execute(api_root_request).await?;

    // 3. Extracts the Link header pointing to the WP.org API root [this needs error handling for "what if this isn't a WP site?"
    let api_root_url = api_root_response
        .get_link_header("https://api.w.org/")
        .ok_or(FindApiUrlsError::LinkHeaderNotFound)?
        .to_string();

    // 4. Fetches the API root, which contains the URL to the login page
    let capabilities_request = WpNetworkRequest {
        method: RequestMethod::GET,
        url: WpEndpointUrl(api_root_url.clone()),
        header_map: HashMap::new(),
        body: None,
    };
    let capabilities_response = request_executor.execute(capabilities_request).await?;
    let mut authentication_map = parse_api_details_response(capabilities_response)
        .map_err(|_| FindApiUrlsError::CapabilitiesResponseError)?
        .authentication;
    let application_password_authentication_url = authentication_map
        .remove("application-password")
        .ok_or(FindApiUrlsError::ApplicationPasswordEndpointNotFound)?
        .endpoints
        .authorization;
    Ok(WpRestApiUrls {
        api_root_url,
        application_password_authentication_url,
    })
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

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
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

#[uniffi::export]
pub fn get_link_header(response: &WpNetworkResponse, name: &str) -> Option<WpRestApiUrl> {
    if let Some(url) = response.get_link_header(name) {
        return Some(url.into());
    }

    None
}

#[uniffi::export]
pub fn parse_api_details_response(response: WpNetworkResponse) -> Result<WpApiDetails, WpApiError> {
    let api_details =
        serde_json::from_slice(&response.body).map_err(|err| WpApiError::ParsingError {
            reason: err.to_string(),
            response: response.body_as_string(),
        })?;
    Ok(api_details)
}
