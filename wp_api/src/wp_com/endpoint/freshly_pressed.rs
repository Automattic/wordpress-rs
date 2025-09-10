use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{freshly_pressed::{FreshlyPressedListParams, FreshlyPressedPostList}, WpComNamespace},
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum FreshlyPressedRequest {
    #[get(url = "/freshly-pressed", params = &FreshlyPressedListParams, output = FreshlyPressedPostList)]
    List,
}

impl DerivedRequest for FreshlyPressedRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::RestV1_2
    }
}
