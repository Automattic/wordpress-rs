use std::collections::HashMap;
use std::str;
use std::sync::Arc;
use url::Url;

use crate::request::endpoint::WpEndpointUrl;
use crate::request::{RequestExecutor, RequestMethod, WpNetworkRequest};

use super::{FindApiUrlsError, WpApiDetails, WpRestApiUrls};

const API_ROOT_LINK_HEADER: &str = "https://api.w.org/";

#[derive(Debug, uniffi::Object)]
struct UniffiWpLoginClient {
    inner: Arc<WpLoginClient>,
}

#[uniffi::export]
impl UniffiWpLoginClient {
    #[uniffi::constructor]
    fn new(request_executor: Arc<dyn RequestExecutor>) -> Self {
        Self {
            inner: WpLoginClient::new(request_executor).into(),
        }
    }

    async fn api_discovery(&self, site_url: &str) -> Result<WpRestApiUrls, FindApiUrlsError> {
        self.inner.api_discovery(site_url).await
    }
}

#[derive(Debug)]
pub struct WpLoginClient {
    request_executor: Arc<dyn RequestExecutor>,
}

impl WpLoginClient {
    pub fn new(request_executor: Arc<dyn RequestExecutor>) -> Self {
        Self { request_executor }
    }

    pub async fn api_discovery(&self, site_url: &str) -> Result<WpRestApiUrls, FindApiUrlsError> {
        let result = Url::parse(site_url);
        match result {
            Ok(url) => self.inner_api_discovery(url).await,
            Err(initial_parse_err) => match initial_parse_err {
                // If the url doesn't have a base, try using `https`
                url::ParseError::RelativeUrlWithoutBase => {
                    if let Ok(url) = Url::parse(format!("https://{}", site_url).as_str()) {
                        self.inner_api_discovery(url).await
                    } else {
                        Err(initial_parse_err.into())
                    }
                }
                _ => Err(initial_parse_err.into()),
            },
        }
    }

    async fn inner_api_discovery(
        &self,
        parsed_site_url: Url,
    ) -> Result<WpRestApiUrls, FindApiUrlsError> {
        let api_root_url = self.fetch_api_root_url(parsed_site_url).await?;
        let api_root_url_as_string = api_root_url.to_string();

        let api_details = self.fetch_wp_api_details(api_root_url).await?.into();
        Ok(WpRestApiUrls {
            api_details,
            api_root_url: api_root_url_as_string,
        })
    }

    // Fetches the site's homepage with a HEAD request, then extracts the Link header pointing
    // to the WP.org API root
    async fn fetch_api_root_url(&self, parsed_site_url: Url) -> Result<Url, FindApiUrlsError> {
        let api_root_request = WpNetworkRequest {
            method: RequestMethod::HEAD,
            url: WpEndpointUrl(parsed_site_url.to_string()),
            header_map: HashMap::new(),
            body: None,
        };
        let api_root_response = self.request_executor.execute(api_root_request).await?;

        api_root_response
            .get_link_header(API_ROOT_LINK_HEADER)
            .first()
            .ok_or(FindApiUrlsError::ApiRootLinkHeaderNotFound {
                header_map: api_root_response.header_map,
            })
            .cloned()
    }

    async fn fetch_wp_api_details(
        &self,
        api_root_url: Url,
    ) -> Result<WpApiDetails, FindApiUrlsError> {
        let api_details_response = self
            .request_executor
            .execute(WpNetworkRequest {
                method: RequestMethod::GET,
                url: api_root_url.into(),
                header_map: HashMap::new(),
                body: None,
            })
            .await?;
        serde_json::from_slice(&api_details_response.body).map_err(|err| {
            FindApiUrlsError::ApiDetailsCouldntBeParsed {
                reason: err.to_string(),
                response: api_details_response.body_as_string(),
            }
        })
    }
}
