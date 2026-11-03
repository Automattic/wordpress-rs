use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{WpComNamespace, WpComSiteId, stats_summary::StatsSummaryResponse},
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsSummaryRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats", output = StatsSummaryResponse)]
    GetStatsSummary,
}

impl DerivedRequest for StatsSummaryRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
