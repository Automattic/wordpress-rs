use crate::wp_com::me::WPComUserInfo;
use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::WpComNamespace,
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum MeRequest {
    #[get(url = "/me", output = WPComUserInfo)]
    Get,
}

impl DerivedRequest for MeRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}
