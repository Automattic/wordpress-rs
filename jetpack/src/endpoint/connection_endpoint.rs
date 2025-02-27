use wp_api::request::endpoint::{AsNamespace, DerivedRequest};
use wp_derive_request_builder::WpDerivedRequest;

use crate::JetpackNamespace;

#[derive(WpDerivedRequest)]
enum ConnectionRequest {
    #[post(url = "/connection/register", params = &crate::connection::JetpackConnectionParams, output = crate::connection::JetpackConnectionRegisterResult)]
    Register,
    #[post(url = "/remote_provision", params =  &crate::connection::JetpackRemoteProvisionParams, output = crate::connection::JetpackRemoteProvisionResult)]
    RemoteProvision,
    #[get(url = "/connection", output = crate::connection::JetpackConnection)]
    Connection,
    #[get(url = "/connection/data", output = crate::connection::JetpackConnectionData)]
    ConnectionData,
    #[get(url = "/connection/check", output = crate::connection::JetpackConnectionCheck)]
    ConnectionCheck,
}

impl DerivedRequest for ConnectionRequest {
    fn namespace() -> impl AsNamespace {
        JetpackNamespace {}
    }
}
