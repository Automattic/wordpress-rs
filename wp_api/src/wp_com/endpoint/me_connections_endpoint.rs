use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::WpComNamespace,
    wp_com::me_connections::MeConnectionsResponse,
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum MeConnectionsRequest {
    #[get(url = "/me/connections", output = MeConnectionsResponse)]
    List,
}

impl DerivedRequest for MeConnectionsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::V2
    }
}
