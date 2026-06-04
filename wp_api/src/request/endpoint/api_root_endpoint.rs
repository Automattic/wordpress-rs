use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::login::WpApiDetails;
use std::sync::Arc;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum ApiRootRequest {
    #[get(url = "/", output = Arc<WpApiDetails>)]
    Get,
}

impl DerivedRequest for ApiRootRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::None
    }
}
