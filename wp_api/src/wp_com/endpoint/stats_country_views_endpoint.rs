use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_country_views::{StatsCountryViewsParams, StatsCountryViewsResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsCountryViewsRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/country-views", params = &StatsCountryViewsParams, output = StatsCountryViewsResponse)]
    GetStatsCountryViews,
}

impl DerivedRequest for StatsCountryViewsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
