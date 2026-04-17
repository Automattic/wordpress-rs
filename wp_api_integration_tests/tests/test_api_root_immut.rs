use wp_api::wp_com::{WpComBaseUrl, endpoint::WpComDotOrgApiUrlResolver};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn test_fetching_api_root() {
    let response = api_client().api_root().get().await.assert_response();

    assert_eq!(response.data.url, "http://localhost");
}

#[tokio::test]
#[parallel]
#[ignore]
async fn test_fetching_wpcom_api_root() {
    let resolver = Arc::new(WpComDotOrgApiUrlResolver::new(
        "mobile.blog".to_string(),
        WpComBaseUrl::Production,
    ));
    let response = WpApiClient::new(
        resolver.clone(),
        WpApiClientDelegate {
            auth_provider: Arc::new(WpAuthenticationProvider::static_with_auth(
                WpAuthentication::None,
            )),
            request_executor: Arc::new(ReqwestRequestExecutor::default()),
            middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
            app_notifier: Arc::new(EmptyAppNotifier),
        },
    )
    .api_root()
    .get()
    .await
    .assert_response();

    assert_eq!(response.data.url, "http://mobiledotblog.wordpress.com");

    // Verify `has_route_for_endpoint` matches canonical endpoints against the
    // real WP.com route key shape (which inserts `/sites/{site_id}/`).
    let details = response.data;
    assert!(details.has_route_for_endpoint(
        resolver.as_ref(),
        "/wp/v2".to_string(),
        "posts".to_string(),
    ));
    assert!(details.has_route_for_endpoint(
        resolver.as_ref(),
        "/wp/v2".to_string(),
        "media".to_string(),
    ));
    assert!(details.has_route_for_endpoint(
        resolver.as_ref(),
        "/wp-block-editor/v1".to_string(),
        "settings".to_string(),
    ));
    assert!(!details.has_route_for_endpoint(
        resolver.as_ref(),
        "/wp/v2".to_string(),
        "fake-endpoint".to_string(),
    ));
}
