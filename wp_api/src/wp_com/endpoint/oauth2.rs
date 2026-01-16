use crate::wp_com::oauth2::TokenRequestParameters;
use crate::wp_com::oauth2::TokenRequestResponse;
use crate::wp_com::oauth2::{TokenValidationParameters, TokenValidationResponse};
use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::WpComNamespace,
};
use wp_derive_request_builder::WpDerivedRequest;

// OAuth2 API endpoints for WordPress.com authentication.
//
// Variants:
// - `FetchInfo`: Validates an existing access token and retrieves information about it.
//   Makes a GET request to `/oauth2/token-info` to check if a token is valid
//   and retrieve associated metadata (client ID, user ID, blog ID, and scope).
//
// - `RequestToken`: Exchanges an authorization code for an access token.
//   Makes a POST request to `/oauth2/token` with the authorization code
//   received from the OAuth2 callback to obtain an access token.
#[derive(WpDerivedRequest)]
enum Oauth2Request {
    #[get(url = "/token-info", params = &TokenValidationParameters, output = TokenValidationResponse)]
    FetchInfo,

    #[post(url = "/token", params = &TokenRequestParameters, output = TokenRequestResponse, form_urlencoded = true)]
    RequestToken,
}

impl DerivedRequest for Oauth2Request {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::Oauth2
    }
}
