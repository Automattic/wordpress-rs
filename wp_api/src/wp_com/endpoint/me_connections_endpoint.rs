use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::me_connections::{
        KeyringConnectionDeleteResponse, KeyringConnectionResponse, KeyringTokenId,
        MeConnectionsResponse,
    },
    wp_com::WpComNamespace,
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum MeConnectionsRequest {
    #[get(url = "/me/connections", output = MeConnectionsResponse)]
    List,
    #[get(url = "/me/connections/<keyring_token_id>", output = KeyringConnectionResponse)]
    Get,
    #[delete(url = "/me/connections/<keyring_token_id>", output = KeyringConnectionDeleteResponse)]
    Delete,
}

impl DerivedRequest for MeConnectionsRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::V2
    }
}
