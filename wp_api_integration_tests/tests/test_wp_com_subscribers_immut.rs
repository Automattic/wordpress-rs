use wp_api::wp_com::subscribers::{ListSubscribersSortField, SubscribersListParams};
use wp_api_integration_tests::{WP_COM_SITE_ID, prelude::*, wp_com_client};

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_subscribers(#[case] params: SubscribersListParams) {
    let subscribers = wp_com_client()
        .subscribers()
        .list_subscribers(&WP_COM_SITE_ID, &params)
        .await
        .assert_response();
    assert!(
        subscribers.data.total > 0,
        "Retrieved no subscribers: {:#?}",
        subscribers
    );
}

#[template]
#[rstest]
#[case::default(SubscribersListParams::default())]
#[case::sort_date_subscribed(generate!(SubscribersListParams, (sort, Some(ListSubscribersSortField::DateSubscribed))))]
#[case::sort_email_address(generate!(SubscribersListParams, (sort, Some(ListSubscribersSortField::EmailAddress))))]
#[case::sort_display_name(generate!(SubscribersListParams, (sort, Some(ListSubscribersSortField::DisplayName))))]
#[case::sort_plan(generate!(SubscribersListParams, (sort, Some(ListSubscribersSortField::Plan))))]
#[case::sort_subscription_status(generate!(SubscribersListParams, (sort, Some(ListSubscribersSortField::SubscriptionStatus))))]
pub fn list_cases(#[case] params: SubscribersListParams) {}
