use crate::{
    api_error::RequestExecutionError,
    date::WpGmtDateTime,
    request::{
        RequestContext, RequestExecutor, RequestMethod, WpMultipartFormRequest, WpNetworkHeaderMap,
        WpNetworkRequest, WpNetworkResponse, endpoint::WpEndpointUrl,
    },
    url_query::{AppendUrlQueryPairs, FromUrlQueryPairs, UrlQueryPairsMap},
};
use std::sync::Arc;
use url::Url;

#[cfg(test)]
pub fn assert_expected_query_pairs(params: impl AppendUrlQueryPairs, expected_query: &str) {
    let mut url = Url::parse("https://example.com").unwrap();
    params.append_query_pairs(&mut url.query_pairs_mut());
    assert_eq!(url.query(), Some(expected_query));
}

#[cfg(test)]
pub fn assert_expected_and_from_query_pairs<P>(params: P, expected_query: &str)
where
    P: AppendUrlQueryPairs + FromUrlQueryPairs + std::fmt::Debug + PartialEq,
{
    let mut url = Url::parse("https://example.com").unwrap();
    params.append_query_pairs(&mut url.query_pairs_mut());
    assert_eq!(url.query(), Some(expected_query));

    let parsed_params = P::from_url_query_pairs(UrlQueryPairsMap::new(
        url.query_pairs().into_iter().collect(),
    ));
    assert_eq!(Some(params), parsed_params);
}

#[cfg(test)]
pub fn unit_test_example_date_as_option() -> Option<WpGmtDateTime> {
    Some(
        "2024-02-09T02:14:13+00:00"
            .parse::<WpGmtDateTime>()
            .expect("Example date is parseable"),
    )
}

#[cfg(test)]
pub fn unit_test_example_date_as_query_value(key: &str) -> String {
    format!("{key}=2024-02-09T02%3A14%3A13%2B00%3A00")
}

/// Fails the test if a request reaches it.
#[cfg(test)]
pub struct UnreachableExecutor;

#[cfg(test)]
#[async_trait::async_trait]
impl RequestExecutor for UnreachableExecutor {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        panic!(
            "No request should be sent, but one was sent to {:?}",
            request.url
        );
    }

    async fn upload(
        &self,
        request: Arc<WpMultipartFormRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        panic!(
            "No request should be sent, but one was sent to {:?}",
            request.url
        );
    }

    async fn sleep(&self, _: u64) {}

    fn cancel(&self, _: Arc<RequestContext>) {}
}

#[cfg(test)]
pub fn wp_network_response_from_json(json: &str, status_code: u32) -> WpNetworkResponse {
    WpNetworkResponse {
        body: json.into(),
        status_code,
        response_header_map: Arc::new(WpNetworkHeaderMap::default()),
        request_url: WpEndpointUrl("http://example.com".to_string()),
        request_method: RequestMethod::GET,
        request_header_map: Arc::new(WpNetworkHeaderMap::default()),
    }
}
