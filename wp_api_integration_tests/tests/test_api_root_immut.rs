
use wp_api::wp_com::{endpoint::WpComDotOrgApiUrlResolver, WpComBaseUrl};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn test_fetching_api_root() {
    let response = api_client()
    .api_root()
    .get()
    .await
    .assert_response();

    assert_eq!(response.data.url, "http://localhost");
}

#[tokio::test]
#[parallel]
async fn test_fetching_wpcom_api_root() {
    let response = WpApiClient::new(
        Arc::new(WpComDotOrgApiUrlResolver::new(
            "mobile.blog".to_string(),
            WpComBaseUrl::Production,
        )),
        WpApiClientDelegate {
            auth_provider: Arc::new(WpAuthenticationProvider::static_with_auth(WpAuthentication::None)),
            request_executor: Arc::new(ReqwestRequestExecutor::default()),
            middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
            app_notifier: Arc::new(EmptyAppNotifier),
        },
    ).api_root().get().await.assert_response();

    assert_eq!(response.data.url, "http://mobiledotblog.wordpress.com");
}
