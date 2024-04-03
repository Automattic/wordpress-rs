use std::fs::read_to_string;

use wp_api::{WPAuthentication, WPContext};
use wp_networking::WPNetworking;

fn main() {
    // A very naive approach just to get things working for now - this whole code will be deleted
    // soon
    let file_contents = read_to_string("test_credentials").unwrap();
    let lines: Vec<&str> = file_contents.lines().collect();
    let url = lines[0];
    let auth_base64_token = lines[1];

    let authentication = WPAuthentication::AuthorizationHeader {
        token: auth_base64_token.into(),
    };

    let wp_networking = WPNetworking::new(url.into(), authentication);

    let wp_request = wp_networking
        .api_helper
        .user_list_request(WPContext::Edit, None);
    println!(
        "{:?}",
        wp_api::parse_user_list_response_with_edit_context(
            &wp_networking.request(wp_request).unwrap()
        )
    );
}
