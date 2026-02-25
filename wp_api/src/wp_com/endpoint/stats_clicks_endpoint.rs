use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_clicks::{StatsClicksParams, StatsClicksResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsClicksRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/clicks", params = &StatsClicksParams, output = StatsClicksResponse)]
    GetStatsClicks,
}

impl DerivedRequest for StatsClicksRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
