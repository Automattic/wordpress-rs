use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_emails_summary::{StatsEmailsSummaryParams, StatsEmailsSummaryResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsEmailsSummaryRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/emails/summary", params = &StatsEmailsSummaryParams, output = StatsEmailsSummaryResponse)]
    GetStatsEmailsSummary,
}

impl DerivedRequest for StatsEmailsSummaryRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
