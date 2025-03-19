use serial_test::parallel;
use std::sync::Arc;
use wp_api::{
    middleware::WpApiMiddlewarePipeline, reqwest_request_executor::ReqwestRequestExecutor,
    wordpress_org::client::WordPressOrgApiClient,
};
use wp_api_integration_tests::{api_client, test_site_url, AssertResponse, TestCredentials};

#[tokio::test]
#[parallel]
async fn plugins_update_check() {
    let plugins = api_client()
        .plugins()
        .list_with_view_context(&Default::default())
        .await
        .assert_response()
        .data;
    assert!(!plugins.is_empty());

    let wp_org_client = WordPressOrgApiClient::new(
        Arc::new(ReqwestRequestExecutor::new(true)),
        Arc::new(WpApiMiddlewarePipeline::default()),
    );
    wp_org_client
        .check_plugin_updates(
            TestCredentials::instance()
                .wordpress_core_version
                .to_string(),
            test_site_url(),
            plugins,
        )
        .await
        .assert_response();
}
