use crate::wp_com::oauth2::{TokenValidationParameters, TokenValidationResponse};
use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::WpComNamespace,
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum Oauth2Request {
    #[get(url = "/token-info", params = &TokenValidationParameters, output = TokenValidationResponse)]
    FetchInfo,
}

impl DerivedRequest for Oauth2Request {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::Oauth2
    }
}
