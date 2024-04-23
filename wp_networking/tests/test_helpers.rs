use base64::prelude::*;
use std::fs::read_to_string;
use wp_api::{UserId, UserListParams, WPApiError, WPAuthentication, WPContext, WPNetworkResponse};

use wp_networking::AsyncWPNetworking;

// The first user is also the current user
pub const FIRST_USER_ID: UserId = UserId(1);
pub const SECOND_USER_ID: UserId = UserId(2);

pub fn wp_networking() -> AsyncWPNetworking {
    let (site_url, username, password) = test_credentials();
    let auth_base64_token = BASE64_STANDARD.encode(format!("{}:{}", username, password));

    let authentication = WPAuthentication::AuthorizationHeader {
        token: auth_base64_token,
    };

    AsyncWPNetworking::new(site_url.into(), authentication)
}

pub fn test_credentials() -> (String, String, String) {
    let file_contents = read_to_string("../test_credentials").unwrap();
    let lines: Vec<&str> = file_contents.lines().collect();
    (
        lines[0].to_string(),
        lines[1].to_string(),
        lines[2].to_string(),
    )
}

pub async fn list_users<F, T>(context: WPContext, params: Option<UserListParams>, parser: F)
where
    F: Fn(WPNetworkResponse) -> Result<T, WPApiError>,
{
    let request = wp_networking()
        .api_helper
        .list_users_request(context, &params);
    parser(wp_networking().async_request(request).await.unwrap()).unwrap();
}

pub async fn retrieve_user<F, T>(user_id: UserId, context: WPContext, parser: F)
where
    F: Fn(WPNetworkResponse) -> Result<T, WPApiError>,
{
    let request = wp_networking()
        .api_helper
        .retrieve_user_request(user_id, context);
    parser(wp_networking().async_request(request).await.unwrap()).unwrap();
}

pub async fn retrieve_me<F, T>(context: WPContext, parser: F)
where
    F: Fn(WPNetworkResponse) -> Result<T, WPApiError>,
{
    let request = wp_networking()
        .api_helper
        .retrieve_current_user_request(context);
    parser(wp_networking().async_request(request).await.unwrap()).unwrap();
}
