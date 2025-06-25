use wp_api::wp_com::subscribers::SubscribersListParams;
use wp_api_integration_tests::{WP_COM_SITE_ID, prelude::*, wp_com_client};

#[tokio::test]
#[parallel]
async fn list_subscribers() {
    let subscribers = wp_com_client()
        .subscribers()
        .list_subscribers(&WP_COM_SITE_ID, &SubscribersListParams::default())
        .await
        .assert_response();
    assert!(
        subscribers.data.total > 0,
        "Retrieved no subscribers: {:#?}",
        subscribers
    );
}
