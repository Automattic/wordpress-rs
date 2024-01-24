use std::fs::read_to_string;

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

    let wp_networking = WPNetworking::new(url.into(), authentication);
    let post_list = wp_networking.list_posts(None).unwrap();
    println!("{:?}", post_list);
}
