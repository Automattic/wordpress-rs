use wp_api::wordpress_org::client::WordPressOrgApiClient;
use wp_api_integration_tests::prelude::*;

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
        Arc::new(ReqwestRequestExecutor::default()),
        Arc::new(WpApiMiddlewarePipeline::default()),
    );
    wp_org_client
        .check_plugin_updates(
            TestCredentials::instance()
                .wordpress_core_version
                .to_string(),
            Arc::new(test_site_url()),
            plugins,
        )
        .await
        .assert_response();
}
