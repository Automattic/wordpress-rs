use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_region_views::{StatsRegionViewsParams, StatsRegionViewsResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsRegionViewsRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/location-views/region", params = &StatsRegionViewsParams, output = StatsRegionViewsResponse)]
    GetStatsRegionViews,
}

impl DerivedRequest for StatsRegionViewsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
