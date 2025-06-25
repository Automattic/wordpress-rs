use wp_api::wp_com::subscribers::{
    GetSubscriberQuery, ListSubscribersSortField, SubscribersListParams, SubscriptionId,
};
use wp_api_integration_tests::{WP_COM_SITE_ID, prelude::*, wp_com_client};

// TODO: Update these for your test site
const WP_COM_SUBSCRIBER_USER_ID: i64 = 0;
const EMAIL_SUBSCRIBER_SUBSCRIPTION_ID: u64 = 0;

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

#[tokio::test]
#[apply(retrieve_cases)]
#[parallel]
async fn retrieve_subscriber(#[case] query: GetSubscriberQuery) {
    wp_com_client()
        .subscribers()
        .get_subscriber(&WP_COM_SITE_ID, &query)
        .await
        .assert_response();
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

#[template]
#[rstest]
#[case::wp_com_subscriber(GetSubscriberQuery::WpCom(UserId(WP_COM_SUBSCRIBER_USER_ID)))]
#[case::email_subscriber(GetSubscriberQuery::Email(SubscriptionId(
    EMAIL_SUBSCRIBER_SUBSCRIPTION_ID
)))]
pub fn retrieve_cases(#[case] query: GetSubscriberQuery) {}
