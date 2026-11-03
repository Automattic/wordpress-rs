use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{WpComNamespace, WpComSiteId, stats_insights::StatsInsightsResponse},
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsInsightsRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/insights", output = StatsInsightsResponse)]
    GetStatsInsights,
}

impl DerivedRequest for StatsInsightsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
