#[derive(wp_derive_request_builder::WpDerivedRequest)]
enum TokenRequest {
    #[post(url = "/token", params = &TokenRequestParams, output = TokenResponse, form_urlencoded = true)]
    RequestToken,
}

fn main() {}
