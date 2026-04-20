use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_visits::{StatsVisitsParams, StatsVisitsResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsVisitsRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/visits", params = &StatsVisitsParams, output = StatsVisitsResponse)]
    GetStatsVisits,
}

impl DerivedRequest for StatsVisitsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
