use std::sync::Arc;

use wp_api::{WPAuthentication, WPNetworkRequest, WPNetworkResponse, WPNetworkingInterface};

fn main() {
    let mock_authentication = WPAuthentication {
        auth_token: "mock_token".into(),
    };

    let post_list = wp_networking::wp_api(mock_authentication.clone()).list_posts(None);
    println!("Post List with built-in networking: {:?}", post_list);

    let post_list_with_custom_networking = wp_networking::wp_api_with_custom_networking(
        mock_authentication.clone(),
        Arc::new(CustomWPNetworking {}),
    )
    .list_posts(None);
    println!(
        "Post List with custom networking: {:?}",
        post_list_with_custom_networking
    );
}

struct CustomWPNetworking {}

impl WPNetworkingInterface for CustomWPNetworking {
    fn request(&self, _request: WPNetworkRequest) -> WPNetworkResponse {
        todo!()
    }
}
