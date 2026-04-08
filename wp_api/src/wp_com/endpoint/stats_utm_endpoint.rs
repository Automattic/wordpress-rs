use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_utm::{StatsUtmKeys, StatsUtmParams, StatsUtmResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsUtmRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/utm/<stats_utm_keys>", params = &StatsUtmParams, output = StatsUtmResponse)]
    GetStatsUtm,
}

impl DerivedRequest for StatsUtmRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
