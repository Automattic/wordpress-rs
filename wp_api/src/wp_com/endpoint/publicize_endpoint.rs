use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        publicize::{
            CreatePublicizeConnectionParams, PublicizeConnectionId, PublicizeConnectionResponse,
            PublicizeServiceResponse, UpdatePublicizeConnectionParams,
        },
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum PublicizeRequest {
    #[get(url = "/sites/<wp_com_site_id>/publicize/connections", output = Vec<PublicizeConnectionResponse>)]
    ListConnections,
    #[get(url = "/sites/<wp_com_site_id>/publicize/services", output = Vec<PublicizeServiceResponse>)]
    ListServices,
    #[post(url = "/sites/<wp_com_site_id>/publicize/connections", params = &CreatePublicizeConnectionParams, output = PublicizeConnectionResponse)]
    CreateConnection,
    #[post(url = "/sites/<wp_com_site_id>/publicize/connections/<publicize_connection_id>", params = &UpdatePublicizeConnectionParams, output = PublicizeConnectionResponse)]
    UpdateConnection,
    #[delete(url = "/sites/<wp_com_site_id>/publicize/connections/<publicize_connection_id>", output = bool)]
    DeleteConnection,
}

impl DerivedRequest for PublicizeRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::V2
    }
}
