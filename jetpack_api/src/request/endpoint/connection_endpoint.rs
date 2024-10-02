use wp_derive_request_builder::WpDerivedRequest;

use super::JetpackNamespace;
use wp_api::request::endpoint::{AsNamespace, DerivedRequest};

#[derive(WpDerivedRequest)]
#[ErrorType(crate::JpApiError)]
enum ConnectionRequest {
    #[get(url = "/connection", output = crate::jetpack_connection::JetpackConnectionStatus)]
    Status,
}

impl DerivedRequest for ConnectionRequest {
    fn namespace() -> impl AsNamespace {
        JetpackNamespace::JetpackV4
    }
}
