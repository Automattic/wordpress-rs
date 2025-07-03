use wp_api::wp_com::{
    WpComSiteId,
    subscribers::{
        IndividualSubscriberParams, IndividualSubscriberStatsParams, ListSubscribersSortField,
        SubscribersListParams, SubscriptionId,
    },
};
use wp_api_integration_tests::{WpComTestCredentials, prelude::*, wp_com_client};

#[tokio::test]
#[apply(list_cases)]
#[parallel]
#[ignore]
async fn list_subscribers(#[case] params: SubscribersListParams) {
    let subscribers = wp_com_client()
        .subscribers()
        .list_subscribers(
            &WpComSiteId(WpComTestCredentials::instance().site_id),
            &params,
        )
        .await
        .assert_response();
    assert!(
        subscribers.data.total > 0,
        "Retrieved no subscribers: {subscribers:#?}"
    );
}

#[tokio::test]
#[apply(retrieve_cases)]
#[parallel]
#[ignore]
async fn retrieve_subscriber(#[case] query: IndividualSubscriberParams) {
    wp_com_client()
        .subscribers()
        .individual_subscriber(
            &WpComSiteId(WpComTestCredentials::instance().site_id),
            &query,
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
#[ignore]
async fn individual_subscriber_stats() {
    wp_com_client()
        .subscribers()
        .individual_subscriber_stats(
            &WpComSiteId(WpComTestCredentials::instance().site_id),
            &IndividualSubscriberStatsParams {
                subscription_id: SubscriptionId(
                    WpComTestCredentials::instance().email_subscriber_subscription_id,
                ),
            },
        )
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
#[case::wp_com_subscriber(IndividualSubscriberParams::WpCom(UserId(WpComTestCredentials::instance().wp_com_subscriber_user_id)))]
#[case::email_subscriber(IndividualSubscriberParams::Email(SubscriptionId(
    WpComTestCredentials::instance().email_subscriber_subscription_id
)))]
pub fn retrieve_cases(#[case] query: IndividualSubscriberParams) {}
