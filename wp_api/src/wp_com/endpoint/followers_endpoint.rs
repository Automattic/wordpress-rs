use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    users::UserId,
    wp_com::{
        WpComNamespace, WpComSiteId, followers::DeleteFollowerResponse, subscribers::SubscriptionId,
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum FollowersRequest {
    #[post(url = "/sites/<wp_com_site_id>/followers/<user_id>/delete", output = DeleteFollowerResponse)]
    DeleteFollower,
    #[post(url = "/sites/<wp_com_site_id>/email-followers/<subscription_id>/delete", output = DeleteFollowerResponse)]
    DeleteEmailFollower,
}

impl DerivedRequest for FollowersRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
