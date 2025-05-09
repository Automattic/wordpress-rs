use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        jetpack_connection::{JetpackRemoteConnectionParams, JetpackRemoteConnectionResult},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum JetpackConnectionRequest {
    #[post(url = "/sites/<wp_com_site_id>/jetpack-remote-connect-user", params = &JetpackRemoteConnectionParams, output = JetpackRemoteConnectionResult)]
    RemoteConnectUser,
}

impl DerivedRequest for JetpackConnectionRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::V2
    }
}
