use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        publicize::{PublicizeConnectionResponse, PublicizeServiceResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum PublicizeRequest {
    #[get(url = "/sites/<wp_com_site_id>/publicize/connections", output = Vec<PublicizeConnectionResponse>)]
    ListConnections,
    #[get(url = "/sites/<wp_com_site_id>/publicize/services", output = Vec<PublicizeServiceResponse>)]
    ListServices,
}

impl DerivedRequest for PublicizeRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::V2
    }
}
