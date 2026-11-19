use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        reader_subscriptions::{
            ReaderCommentEmailSubscriptionResponse, ReaderFollowSiteResponse,
            ReaderFollowUrlParams, ReaderFollowUrlResponse, ReaderNotificationSubscriptionResponse,
            ReaderPostEmailFrequencyParams, ReaderPostEmailSubscriptionParams,
            ReaderPostEmailSubscriptionResponse, ReaderSiteFollowStatus, ReaderSubscriptionsParams,
            ReaderSubscriptionsResponse, ReaderUnfollowUrlParams, ReaderUnfollowUrlResponse,
        },
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum ReaderSubscriptionsRequest {
    // The sites and feeds the user is subscribed to, by page number. Needs a token.
    #[get(url = "/read/following/mine", params = &ReaderSubscriptionsParams, output = ReaderSubscriptionsResponse)]
    List,
    // Whether the user follows a site. Needs a token. It takes a site's ID, not a feed's: a feed's
    // ID is read as some other site's.
    #[get(url = "/sites/<wp_com_site_id>/follows/mine", output = ReaderSiteFollowStatus)]
    IsFollowingSite,
    // Follows a WordPress.com or Jetpack site, by its ID, and answers as an unfollow does. For a
    // site the user follows already it's a 400 `already_subscribed`.
    //
    // When this was first tried, the API refused every follow with a 400 `follow_failed` and an
    // empty message. A day later it took them, with nothing changed on this side.
    //
    // The API declares a `source` for it, which says where the follow came from, and reads it
    // from the address only: the apps add `?source=android` or `?source=ios`. A POST here sends
    // its parameters in the body, so no `source` is sent. A follow by address takes one.
    #[post(url = "/sites/<wp_com_site_id>/follows/new", output = ReaderFollowSiteResponse)]
    FollowSite,
    // Unfollows a site, by its ID. It takes no parameters. For a site the user doesn't follow
    // it's a 400 `are_not_subscribed`.
    #[post(url = "/sites/<wp_com_site_id>/follows/mine/delete", output = ReaderFollowSiteResponse)]
    UnfollowSite,
    // Follows a site or a feed, by its address, and answers with the new subscription. An
    // address with no feed is still a 200, with `subscribed: false` and `info: "invalid_feed"`.
    #[post(url = "/read/following/mine/new", params = &ReaderFollowUrlParams, output = ReaderFollowUrlResponse)]
    FollowUrl,
    // Unfollows a site or a feed, by its subscription's ID or by the address that was followed.
    #[post(url = "/read/following/mine/delete", params = &ReaderUnfollowUrlParams, output = ReaderUnfollowUrlResponse)]
    UnfollowUrl,
    // Starts emailing the user a site's new posts.
    #[post(url = "/read/site/<wp_com_site_id>/post_email_subscriptions/new", params = &ReaderPostEmailSubscriptionParams, output = ReaderPostEmailSubscriptionResponse)]
    SubscribeToPostEmails,
    // Stops emailing the user a site's new posts.
    #[post(url = "/read/site/<wp_com_site_id>/post_email_subscriptions/delete", params = &ReaderPostEmailSubscriptionParams, output = ReaderPostEmailSubscriptionResponse)]
    UnsubscribeFromPostEmails,
    // Changes how often a site's new posts are emailed. A frequency the API doesn't know is
    // still a 200 with `success: true`, and sets the emails to `instantly`.
    #[post(url = "/read/site/<wp_com_site_id>/post_email_subscriptions/update", params = &ReaderPostEmailFrequencyParams, output = ReaderPostEmailSubscriptionResponse)]
    UpdatePostEmails,
    // Starts emailing the user the new comments on a site's posts.
    //
    // The API declares a `post_id` for this and for stopping them, which it reads from the
    // address only. A POST here sends its parameters in the body, so no `post_id` is sent, and
    // both act on the whole site.
    #[post(url = "/read/site/<wp_com_site_id>/comment_email_subscriptions/new", output = ReaderCommentEmailSubscriptionResponse)]
    SubscribeToCommentEmails,
    // Stops emailing the user the new comments on a site's posts.
    #[post(url = "/read/site/<wp_com_site_id>/comment_email_subscriptions/delete", output = ReaderCommentEmailSubscriptionResponse)]
    UnsubscribeFromCommentEmails,
    // Turns on notifications of a site's new posts. It takes no parameters.
    #[post(url = "/read/sites/<wp_com_site_id>/notification-subscriptions/new", output = ReaderNotificationSubscriptionResponse)]
    SubscribeToNotifications,
    // Turns off notifications of a site's new posts. It takes no parameters.
    #[post(url = "/read/sites/<wp_com_site_id>/notification-subscriptions/delete", output = ReaderNotificationSubscriptionResponse)]
    UnsubscribeFromNotifications,
}

impl DerivedRequest for ReaderSubscriptionsRequest {
    fn namespace(&self) -> impl AsNamespace {
        match self {
            // The newest list. `rest/v1.1` returns every subscription at once, without paging.
            Self::List => WpComNamespace::RestV1_2,
            // The versions the apps and the web Reader use, and the newest of each.
            Self::SubscribeToPostEmails
            | Self::UnsubscribeFromPostEmails
            | Self::UpdatePostEmails
            | Self::SubscribeToCommentEmails
            | Self::UnsubscribeFromCommentEmails => WpComNamespace::RestV1_2,
            Self::SubscribeToNotifications | Self::UnsubscribeFromNotifications => {
                WpComNamespace::V2
            }
            // The only version of each: `rest/v1.2` has none of them.
            Self::IsFollowingSite
            | Self::FollowSite
            | Self::UnfollowSite
            | Self::FollowUrl
            | Self::UnfollowUrl => WpComNamespace::RestV1_1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::ApiUrlResolver,
        wp_com::endpoint::tests::{
            fixture_wp_com_api_url_resolver, validate_wp_com_rest_v1_1_endpoint,
            validate_wp_com_rest_v1_2_endpoint, validate_wp_com_v2_endpoint,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case::default(ReaderSubscriptionsParams::default(), "/read/following/mine?")]
    #[case::all_params(
        ReaderSubscriptionsParams {
            page: Some(2),
            number: Some(100),
            q: Some("example blog".to_string()),
            filter: Some("a8c".to_string()),
            meta: Some("site,feed".to_string()),
        },
        "/read/following/mine?page=2&number=100&q=example+blog&filter=a8c&meta=site%2Cfeed"
    )]
    fn list(
        endpoint: ReaderSubscriptionsRequestEndpoint,
        #[case] params: ReaderSubscriptionsParams,
        #[case] expected_path: &str,
    ) {
        validate_wp_com_rest_v1_2_endpoint(endpoint.list(&params), expected_path);
    }

    #[rstest]
    fn is_following_site(endpoint: ReaderSubscriptionsRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.is_following_site(&WpComSiteId(3584907)),
            "/sites/3584907/follows/mine",
        );
    }

    #[rstest]
    fn follow_and_unfollow_site(endpoint: ReaderSubscriptionsRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.follow_site(&WpComSiteId(3584907)),
            "/sites/3584907/follows/new",
        );
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.unfollow_site(&WpComSiteId(3584907)),
            "/sites/3584907/follows/mine/delete",
        );
    }

    #[rstest]
    fn follow_and_unfollow_url(endpoint: ReaderSubscriptionsRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(endpoint.follow_url(), "/read/following/mine/new");
        validate_wp_com_rest_v1_1_endpoint(endpoint.unfollow_url(), "/read/following/mine/delete");
    }

    /// The emails are under `read/site`, and the notifications under `read/sites`.
    #[rstest]
    fn delivery_settings(endpoint: ReaderSubscriptionsRequestEndpoint) {
        let site = WpComSiteId(3584907);
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.subscribe_to_post_emails(&site),
            "/read/site/3584907/post_email_subscriptions/new",
        );
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.unsubscribe_from_post_emails(&site),
            "/read/site/3584907/post_email_subscriptions/delete",
        );
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.update_post_emails(&site),
            "/read/site/3584907/post_email_subscriptions/update",
        );
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.subscribe_to_comment_emails(&site),
            "/read/site/3584907/comment_email_subscriptions/new",
        );
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.unsubscribe_from_comment_emails(&site),
            "/read/site/3584907/comment_email_subscriptions/delete",
        );
        validate_wp_com_v2_endpoint(
            endpoint.subscribe_to_notifications(&site),
            "/read/sites/3584907/notification-subscriptions/new",
        );
        validate_wp_com_v2_endpoint(
            endpoint.unsubscribe_from_notifications(&site),
            "/read/sites/3584907/notification-subscriptions/delete",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> ReaderSubscriptionsRequestEndpoint {
        ReaderSubscriptionsRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
