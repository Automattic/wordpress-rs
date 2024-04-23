use base64::prelude::*;
use std::fs::read_to_string;
use wp_api::{UserId, WPAuthentication};

use wp_networking::AsyncWPNetworking;

// The first user is also the current user
pub const FIRST_USER_ID: UserId = UserId(1);
pub const SECOND_USER_ID: UserId = UserId(2);

pub fn wp_networking() -> AsyncWPNetworking {
    let file_contents = read_to_string("../test_credentials").unwrap();
    let lines: Vec<&str> = file_contents.lines().collect();
    let site_url = lines[0];
    let auth_base64_token = BASE64_STANDARD.encode(format!("{}:{}", lines[1], lines[2]));

    let authentication = WPAuthentication::AuthorizationHeader {
        token: auth_base64_token,
    };

    AsyncWPNetworking::new(site_url.into(), authentication)
}
