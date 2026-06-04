use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_tags::{StatsTagsParams, StatsTagsResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsTagsRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/tags", params = &StatsTagsParams, output = StatsTagsResponse)]
    GetStatsTags,
}

impl DerivedRequest for StatsTagsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
