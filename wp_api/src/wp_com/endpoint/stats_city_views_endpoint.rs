use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_city_views::{StatsCityViewsParams, StatsCityViewsResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsCityViewsRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/location-views/city", params = &StatsCityViewsParams, output = StatsCityViewsResponse)]
    GetStatsCityViews,
}

impl DerivedRequest for StatsCityViewsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
