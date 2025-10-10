use crate::wp_com::sites::{SitesListParams, WPComSiteListResponse};
use crate::wp_com::{WpComSiteId, sites::WpComSiteSlug};
use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{WpComNamespace, sites::WPComSite},
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum SitesRequest {
    #[get(url = "/me/sites", params = &SitesListParams, output = WPComSiteListResponse)]
    Get,

    #[get(url = "/sites/<wp_com_site_id>", output = WPComSite)]
    GetSiteById,

    #[get(url = "/sites/<wp_com_site_slug>", output = WPComSite)]
    GetSiteBySlug,
}

impl DerivedRequest for SitesRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::RestV1_2
    }
}
