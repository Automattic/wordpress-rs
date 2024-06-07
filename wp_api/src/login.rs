use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;
use std::sync::Arc;
use url::Url;

use crate::request::RequestExecutor;
use crate::{WpApiError, WpAuthentication};

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum FindApiUrlsError {
    #[error("Not yet implemented")]
    NotYetImplemented,
}

#[derive(uniffi::Record)]
pub struct WpRestApiRootUrl {}

#[derive(uniffi::Object)]
pub struct WpRestApiUrls {}

impl WpRestApiUrls {
    fn login_url_as_str(&self) -> &str {
        todo!()
    }

    fn api_root_url(&self) -> WpRestApiRootUrl {
        todo!()
    }
}

// We'll use original request builder, moved here as a showcase
#[derive(uniffi::Object)]
struct WpRequestBuilder {}

#[uniffi::export]
impl WpRequestBuilder {
    #[uniffi::constructor]
    pub fn new(
        site_url: WpRestApiRootUrl,
        authentication: WpAuthentication,
        request_executor: Arc<dyn RequestExecutor>,
    ) -> Result<Self, WpApiError> {
        todo!()
    }
}

#[uniffi::export]
pub fn find_api_urls(
    site_url: String,
    request_executor: Arc<dyn RequestExecutor>,
) -> Result<WpRestApiUrls, FindApiUrlsError> {
    // 1. Parse the URL to standardize its format (so "example.com" would become "https://example.com")
    // let parsedUrl = try WordPressAPI.Helpers.parseUrl(string: url)
    //
    // 2. Fetches the site's homepage with a HEAD request
    // guard let apiRoot = try await WordPressAPI.findRestApiEndpointRoot(
    //     forSiteUrl: parsedUrl,
    //     using: URLSession.shared
    // ) else {
    //     return nil
    // }
    // func findRestApiEndpointRoot(forSiteUrl url: URL, using session: URLSession) async throws -> URL? {
    //     let request = WpNetworkRequest(method: .head, url: url, headerMap: [:])
    //     let ephemeralClient = try WordPressAPI(urlSession: session, baseUrl: url, authenticationStategy: .none)
    //     let response = try await ephemeralClient.perform(request: request)
    //
    // 3. Extracts the Link header pointing to the WP.org API root [this needs error handling for "what if this isn't a WP site?"
    //     return getLinkHeader(response: response, name: "https://api.w.org/")?.asUrl()
    // }
    //
    // 4. Fetches the API root, which contains the URL to the login page
    // let capabilities = try await client.getRestAPICapabilities(forApiRoot: apiRoot, using: .shared)
    // func getRestAPICapabilities(forApiRoot url: URL, using session: URLSession) async throws -> WpApiDetails {
    //     let wpResponse = try await self.perform(request: WpNetworkRequest(method: .get, url: url, headerMap: [:]))
    //     return try parseApiDetailsResponse(response: wpResponse)
    // }
    // guard let authenticationUrl = capabilities.authentication.first?.value.endpoints.authorization else {
    //     debugPrint("No authentication approaches found – unable to continue")
    //     abort()
    // }
    // return URL(string: authenticationUrl)
    //
    todo!()
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
