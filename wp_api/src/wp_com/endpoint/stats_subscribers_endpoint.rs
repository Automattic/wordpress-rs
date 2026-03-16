use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_subscribers::{StatsSubscribersParams, StatsSubscribersResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsSubscribersRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/subscribers", params = &StatsSubscribersParams, output = StatsSubscribersResponse)]
    GetStatsSubscribers,
}

impl DerivedRequest for StatsSubscribersRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
