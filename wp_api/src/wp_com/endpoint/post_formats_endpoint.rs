use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{WpComNamespace, WpComSiteId},
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum PostFormatsRequest {
    #[get(url = "/sites/<wp_com_site_id>/post-formats", output = crate::wp_com::post_formats::WpComPostFormatsResponse)]
    Get,
}

impl DerivedRequest for PostFormatsRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
