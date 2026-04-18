use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_search_terms::{StatsSearchTermsParams, StatsSearchTermsResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsSearchTermsRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/search-terms", params = &StatsSearchTermsParams, output = StatsSearchTermsResponse)]
    GetStatsSearchTerms,
}

impl DerivedRequest for StatsSearchTermsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
