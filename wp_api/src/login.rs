use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;
use std::sync::Arc;
use url::Url;

use crate::parser::{ParseSiteUrlError, ParsedSiteUrl};
use crate::request::endpoint::WpEndpointUrl;
use crate::request::{RequestExecutor, RequestMethod, WpNetworkRequest};
use crate::RequestExecutionError;

const API_ROOT_LINK_HEADER: &str = "https://api.w.org/";
const KEY_APPLICATION_PASSWORDS: &str = "application-passwords";

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum FindApiUrlsError {
    #[error("Api details couldn't be parsed from response: {:?}", response)]
    ApiDetailsCouldntBeParsed { response: String },
    #[error("Api root link header not found in response: {:?}", response)]
    ApiRootLinkHeaderNotFound { response: String },
    #[error(transparent)]
    ParseSiteUrlError {
        #[from]
        inner: ParseSiteUrlError,
    },
    #[error(transparent)]
    RequestExecutionError {
        #[from]
        inner: RequestExecutionError,
    },
}

#[derive(Debug, uniffi::Record)]
pub struct WpRestApiUrls {
    api_details: Arc<WpApiDetails>,
    api_root_url: String,
}

#[uniffi::export]
pub async fn find_api_urls(
    site_url: String,
    request_executor: Arc<dyn RequestExecutor>,
) -> Result<WpRestApiUrls, FindApiUrlsError> {
    let parsed_site_url = ParsedSiteUrl::parse_str(site_url)?;

    let api_root_url = fetch_api_root_url(&parsed_site_url, &request_executor)
        .await?
        .to_string();

    let api_details = fetch_wp_api_details(&api_root_url, &request_executor)
        .await?
        .into();
    Ok(WpRestApiUrls {
        api_details,
        api_root_url,
    })
}

// Fetches the site's homepage with a HEAD request, then extracts the Link header pointing
// to the WP.org API root
async fn fetch_api_root_url(
    parsed_site_url: &ParsedSiteUrl,
    request_executor: &Arc<dyn RequestExecutor>,
) -> Result<Url, FindApiUrlsError> {
    let api_root_request = WpNetworkRequest {
        method: RequestMethod::HEAD,
        url: WpEndpointUrl(parsed_site_url.site_url.to_string()),
        header_map: HashMap::new(),
        body: None,
    };
    let api_root_response = request_executor.execute(api_root_request).await?;

    api_root_response
        .get_link_header(API_ROOT_LINK_HEADER)
        .ok_or(FindApiUrlsError::ApiRootLinkHeaderNotFound {
            response: api_root_response.body_as_string(),
        })
}

async fn fetch_wp_api_details(
    api_root_url: &String,
    request_executor: &Arc<dyn RequestExecutor>,
) -> Result<WpApiDetails, FindApiUrlsError> {
    let api_details_response = request_executor
        .execute(WpNetworkRequest {
            method: RequestMethod::GET,
            url: WpEndpointUrl(api_root_url.clone()),
            header_map: HashMap::new(),
            body: None,
        })
        .await?;
    serde_json::from_slice(&api_details_response.body).map_err(|_| {
        FindApiUrlsError::ApiDetailsCouldntBeParsed {
            response: api_details_response.body_as_string(),
        }
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
