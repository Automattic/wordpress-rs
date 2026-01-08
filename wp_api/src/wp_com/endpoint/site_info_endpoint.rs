use crate::wp_com::site_info::{SiteInfoParameters, SiteInfoResponse};
use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::WpComNamespace,
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum SiteInfoRequest {
    #[get(url = "/connect/site-info", params = &SiteInfoParameters, output = SiteInfoResponse)]
    Fetch,
}

impl DerivedRequest for SiteInfoRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
