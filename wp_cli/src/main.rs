use std::{fs::read_to_string, sync::Arc};

use wp_api::WPAuthentication;
use wp_networking::WPNetworking;

fn main() {
    // A very naive approach just to get things working for now - this whole code will be deleted
    // soon
    let file_contents = read_to_string("test_credentials").unwrap();
    let lines: Vec<&str> = file_contents.lines().collect();
    let url = lines[0];
    let auth_base64_token = lines[1];

    let authentication = WPAuthentication {
        auth_token: auth_base64_token.into(),
    };

    let post_list = wp_networking::wp_api_with_custom_networking(
        url.into(),
        authentication.clone(),
        Arc::new(WPNetworking::default()),
    )
    .list_posts(None)
    .unwrap();
    println!("{:?}", post_list);

    // let post_list_with_custom_networking = wp_networking::wp_api_with_custom_networking(
    //     mock_authentication.clone(),
    //     Arc::new(CustomWPNetworking {}),
    // )
    // .list_posts(None);
    // println!(
    //     "Post List with custom networking: {:?}",
    //     post_list_with_custom_networking
    // );
}

// struct CustomWPNetworking {}
//
// impl WPNetworkingInterface for CustomWPNetworking {
//     fn request(&self, _request: WPNetworkRequest) -> WPNetworkResponse {
//         todo!()
//     }
// }
//
//
