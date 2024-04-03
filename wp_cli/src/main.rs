use std::fs::read_to_string;

use wp_api::{UserRetrieveParams, WPAuthentication, WPContext};
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

    let user_list_request = wp_networking
        .api_helper
        .user_list_request(WPContext::Edit, None);
    let user_list = wp_api::parse_user_list_response_with_edit_context(
        &wp_networking.request(user_list_request).unwrap(),
    )
    .unwrap();
    println!("{:?}", user_list);

    if let Some(first_user) = user_list.first() {
        let user_retrieve_request = wp_networking
            .api_helper
            .user_retrieve_request(WPContext::Embed, UserRetrieveParams { id: first_user.id });
        println!(
            "{:?}",
            wp_api::parse_user_retrieve_response_with_embed_context(
                &wp_networking.request(user_retrieve_request).unwrap()
            )
        );
    }
}
