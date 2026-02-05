use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_top_authors::{StatsTopAuthorsParams, StatsTopAuthorsResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsTopAuthorsRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/top-authors", params = &StatsTopAuthorsParams, output = StatsTopAuthorsResponse)]
    GetStatsTopAuthors,
}

impl DerivedRequest for StatsTopAuthorsRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
