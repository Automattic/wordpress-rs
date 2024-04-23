use base64::prelude::*;
use std::fs::read_to_string;
use wp_api::{UserId, UserListParams, WPApiError, WPAuthentication, WPContext, WPNetworkResponse};

use wp_networking::AsyncWPNetworking;

// The first user is also the current user
const FIRST_USER_ID: UserId = UserId(1);

#[tokio::test]
async fn list_users_with_edit_context() {
    test_list_users_helper(WPContext::Edit, None, |r| {
        wp_api::parse_list_users_response_with_edit_context(&r)
    })
    .await
}

#[tokio::test]
async fn list_users_with_embed_context() {
    test_list_users_helper(WPContext::Embed, None, |r| {
        wp_api::parse_list_users_response_with_embed_context(&r)
    })
    .await
}

#[tokio::test]
async fn list_users_with_view_context() {
    test_list_users_helper(WPContext::View, None, |r| {
        wp_api::parse_list_users_response_with_view_context(&r)
    })
    .await
}

#[tokio::test]
async fn list_users_with_edit_context_second_page() {
    let params = UserListParams {
        page: Some(2),
        per_page: Some(2),
        search: None,
        exclude: None,
        include: None,
        offset: None,
        order: None,
        order_by: None,
        slug: Vec::new(),
        roles: Vec::new(),
        capabilities: Vec::new(),
        who: None,
        has_published_posts: None,
    };
    test_list_users_helper(WPContext::Edit, Some(params), |r| {
        wp_api::parse_list_users_response_with_edit_context(&r)
    })
    .await
}

#[tokio::test]
async fn retrieve_user_with_edit_context() {
    test_retrieve_user_helper(FIRST_USER_ID, WPContext::Edit, |r| {
        wp_api::parse_retrieve_user_response_with_edit_context(&r)
    })
    .await
}

#[tokio::test]
async fn retrieve_user_with_embed_context() {
    test_retrieve_user_helper(FIRST_USER_ID, WPContext::Embed, |r| {
        wp_api::parse_retrieve_user_response_with_embed_context(&r)
    })
    .await
}

#[tokio::test]
async fn retrieve_user_with_view_context() {
    test_retrieve_me_helper(WPContext::View, |r| {
        wp_api::parse_retrieve_user_response_with_view_context(&r)
    })
    .await
}

#[tokio::test]
async fn retrieve_me_with_edit_context() {
    test_retrieve_me_helper(WPContext::Edit, |r| {
        wp_api::parse_retrieve_user_response_with_edit_context(&r)
    })
    .await
}

#[tokio::test]
async fn retrieve_me_with_embed_context() {
    test_retrieve_me_helper(WPContext::Embed, |r| {
        wp_api::parse_retrieve_user_response_with_embed_context(&r)
    })
    .await
}

#[tokio::test]
async fn retrieve_me_with_view_context() {
    test_retrieve_user_helper(FIRST_USER_ID, WPContext::View, |r| {
        wp_api::parse_retrieve_user_response_with_view_context(&r)
    })
    .await
}

fn wp_networking() -> AsyncWPNetworking {
    let file_contents = read_to_string("../test_credentials").unwrap();
    let lines: Vec<&str> = file_contents.lines().collect();
    let site_url = lines[0];
    let auth_base64_token = BASE64_STANDARD.encode(format!("{}:{}", lines[1], lines[2]));

    let authentication = WPAuthentication::AuthorizationHeader {
        token: auth_base64_token,
    };

    AsyncWPNetworking::new(site_url.into(), authentication)
}

async fn test_list_users_helper<F, T>(context: WPContext, params: Option<UserListParams>, parser: F)
where
    F: Fn(WPNetworkResponse) -> Result<T, WPApiError>,
{
    let request = wp_networking()
        .api_helper
        .list_users_request(context, &params);
    parser(wp_networking().async_request(request).await.unwrap()).unwrap();
}

async fn test_retrieve_user_helper<F, T>(user_id: UserId, context: WPContext, parser: F)
where
    F: Fn(WPNetworkResponse) -> Result<T, WPApiError>,
{
    let request = wp_networking()
        .api_helper
        .retrieve_user_request(user_id, context);
    parser(wp_networking().async_request(request).await.unwrap()).unwrap();
}

async fn test_retrieve_me_helper<F, T>(context: WPContext, parser: F)
where
    F: Fn(WPNetworkResponse) -> Result<T, WPApiError>,
{
    let request = wp_networking()
        .api_helper
        .retrieve_current_user_request(context);
    parser(wp_networking().async_request(request).await.unwrap()).unwrap();
}
