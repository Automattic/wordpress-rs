use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_referrers::{StatsReferrersParams, StatsReferrersResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsReferrersRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/referrers", params = &StatsReferrersParams, output = StatsReferrersResponse)]
    GetStatsReferrers,
}

impl DerivedRequest for StatsReferrersRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
