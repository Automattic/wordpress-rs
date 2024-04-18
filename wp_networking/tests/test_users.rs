use base64::prelude::*;
use std::fs::read_to_string;
use wp_api::{
    UserCreateParamsBuilder, UserId, UserListParams, UserUpdateParamsBuilder, WPApiError,
    WPAuthentication, WPContext, WPNetworkResponse,
};

use wp_networking::AsyncWPNetworking;

mod wp_db;

const FIRST_USER_ID: UserId = UserId(1);

#[tokio::test]
async fn immut_test_list_users_with_edit_context() {
    test_list_users_helper(WPContext::Edit, None, |r| {
        wp_api::parse_list_users_response_with_edit_context(&r)
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn immut_test_list_users_with_embed_context() {
    test_list_users_helper(WPContext::Embed, None, |r| {
        wp_api::parse_list_users_response_with_embed_context(&r)
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn immut_test_list_users_with_view_context() {
    test_list_users_helper(WPContext::View, None, |r| {
        wp_api::parse_list_users_response_with_view_context(&r)
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn immut_test_list_users_with_edit_context_second_page() {
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
    .unwrap();
}

#[tokio::test]
async fn immut_test_retrieve_user_with_edit_context() {
    test_retrieve_user_helper(FIRST_USER_ID, WPContext::Edit, |r| {
        wp_api::parse_retrieve_user_response_with_edit_context(&r)
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn immut_test_retrieve_user_with_embed_context() {
    test_retrieve_user_helper(FIRST_USER_ID, WPContext::Embed, |r| {
        wp_api::parse_retrieve_user_response_with_embed_context(&r)
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn immut_test_retrieve_user_with_view_context() {
    test_retrieve_me_helper(WPContext::View, |r| {
        wp_api::parse_retrieve_user_response_with_view_context(&r)
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn immut_test_retrieve_me_with_edit_context() {
    test_retrieve_me_helper(WPContext::Edit, |r| {
        wp_api::parse_retrieve_user_response_with_edit_context(&r)
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn immut_test_retrieve_me_with_embed_context() {
    test_retrieve_me_helper(WPContext::Embed, |r| {
        wp_api::parse_retrieve_user_response_with_embed_context(&r)
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn immut_test_retrieve_me_with_view_context() {
    test_retrieve_user_helper(FIRST_USER_ID, WPContext::View, |r| {
        wp_api::parse_retrieve_user_response_with_view_context(&r)
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn mut_test_create_user() {
    wp_db::run_and_restore(|mut db| async move {
        let username = "t_username";
        let email = "t_email@foo.com";

        // Create a user using the API
        let user_create_params = UserCreateParamsBuilder::default()
            .username(username.to_string())
            .email(email.to_string())
            .password("t_password".to_string())
            .build()
            .unwrap();
        let user_create_request = wp_networking()
            .api_helper
            .create_user_request(&user_create_params);
        let user_create_response = wp_networking().async_request(user_create_request).await;
        assert!(user_create_response.is_ok());
        let created_user =
            wp_api::parse_retrieve_user_response_with_edit_context(&user_create_response.unwrap())
                .unwrap();

        // Assert that the user is in DB
        let created_user_from_db = db.fetch_db_user(created_user.id.0 as u64).await.unwrap();
        assert_eq!(created_user_from_db.username, username);
        assert_eq!(created_user_from_db.email, email);
    })
    .await;
}

#[tokio::test]
async fn mut_test_update_user() {
    wp_db::run_and_restore(|mut db| async move {
        let new_slug = "new_slug";

        // Update the user's slug using the API and ensure it's successful
        let user_update_params = UserUpdateParamsBuilder::default()
            .slug(Some(new_slug.to_string()))
            .build()
            .unwrap();
        let user_update_request = wp_networking()
            .api_helper
            .update_user_request(FIRST_USER_ID, &user_update_params);
        let user_update_response = wp_networking().async_request(user_update_request).await;
        assert!(user_update_response.is_ok());

        // Assert that the DB record of the user is updated with the new slug
        let first_user_after_update = db.fetch_db_user(FIRST_USER_ID.0 as u64).await.unwrap();
        assert_eq!(first_user_after_update.slug, new_slug);
    })
    .await;
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

async fn test_list_users_helper<F, T>(
    context: WPContext,
    params: Option<UserListParams>,
    parser: F,
) -> Result<T, WPApiError>
where
    F: Fn(WPNetworkResponse) -> Result<T, WPApiError>,
{
    let request = wp_networking()
        .api_helper
        .list_users_request(context, &params);
    parser(wp_networking().async_request(request).await.unwrap())
}

async fn test_retrieve_user_helper<F, T>(
    user_id: UserId,
    context: WPContext,
    parser: F,
) -> Result<T, WPApiError>
where
    F: Fn(WPNetworkResponse) -> Result<T, WPApiError>,
{
    let request = wp_networking()
        .api_helper
        .retrieve_user_request(user_id, context);
    parser(wp_networking().async_request(request).await.unwrap())
}

async fn test_retrieve_me_helper<F, T>(context: WPContext, parser: F) -> Result<T, WPApiError>
where
    F: Fn(WPNetworkResponse) -> Result<T, WPApiError>,
{
    let request = wp_networking()
        .api_helper
        .retrieve_current_user_request(context);
    parser(wp_networking().async_request(request).await.unwrap())
}
