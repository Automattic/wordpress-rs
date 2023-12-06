use std::sync::Arc;

use wp_api::{WPNetworkRequest, WPNetworkResponse, WPNetworkingInterface};

fn main() {
    let post_list = wp_networking::wp_api().list_posts(None);
    println!("Post List with built-in networking: {:?}", post_list);

    let post_list_with_custom_networking =
        wp_networking::wp_api_with_custom_networking(Arc::new(CustomWPNetworking {}))
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
