use crate::{
    jetpack::JetpackNamespace,
    request::endpoint::{AsNamespace, DerivedRequest},
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum ConnectionRequest {
    #[post(url = "/connection/register", params = &crate::jetpack::connection::JetpackConnectionParams, output = crate::jetpack::connection::JetpackConnectionRegisterResult)]
    Register,
    #[post(url = "/remote_provision", params =  &crate::jetpack::connection::JetpackRemoteProvisionParams, output = crate::jetpack::connection::JetpackRemoteProvisionResult)]
    RemoteProvision,
    #[get(url = "/connection", output = crate::jetpack::connection::JetpackConnection)]
    Connection,
    #[get(url = "/connection/data", output = crate::jetpack::connection::JetpackConnectionData)]
    ConnectionData,
    #[get(url = "/connection/check", output = crate::jetpack::connection::JetpackConnectionCheck)]
    ConnectionCheck,
}

impl DerivedRequest for ConnectionRequest {
    fn namespace(&self) -> impl AsNamespace {
        JetpackNamespace {}
    }
}
