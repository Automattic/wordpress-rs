use crate::wp_com::sites::{SitesListParams, WPComSiteListResponse, WpComSiteIdentifier};
use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{WpComNamespace, sites::WPComSite, sites::WPComSiteSummary},
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum SitesRequest {
    #[get(url = "/me/sites", params = &SitesListParams, output = WPComSiteListResponse)]
    Get,

    #[get(url = "/sites/<wp_com_site_identifier>", output = WPComSite)]
    GetSite,

    // Suitable for fetching anonymously
    #[get(url = "/sites/<wp_com_site_identifier>", output = WPComSiteSummary)]
    GetSiteSummary,
}

impl DerivedRequest for SitesRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_2
    }
}
