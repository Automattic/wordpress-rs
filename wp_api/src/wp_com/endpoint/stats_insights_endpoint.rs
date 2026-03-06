use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_insights::{StatsInsightsParams, StatsInsightsResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsInsightsRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/insights", params = &StatsInsightsParams, output = StatsInsightsResponse)]
    GetStatsInsights,
}

impl DerivedRequest for StatsInsightsRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
