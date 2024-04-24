use wp_api::{UserListParams, WPContext};

use crate::test_helpers::{api, WPNetworkRequestExecutor, WPNetworkResponseParser, FIRST_USER_ID};

pub mod test_helpers;

#[tokio::test]
async fn list_users_with_edit_context() {
    assert!(api()
        .list_users_request(WPContext::Edit, &None)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_list_users_response_with_edit_context)
        .is_ok());
}

#[tokio::test]
async fn list_users_with_embed_context() {
    assert!(api()
        .list_users_request(WPContext::Embed, &None)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_list_users_response_with_embed_context)
        .is_ok());
}

#[tokio::test]
async fn list_users_with_view_context() {
    assert!(api()
        .list_users_request(WPContext::View, &None)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_list_users_response_with_view_context)
        .is_ok());
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
    assert!(api()
        .list_users_request(WPContext::Edit, &Some(params))
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_list_users_response_with_edit_context)
        .is_ok());
}

#[tokio::test]
async fn retrieve_user_with_edit_context() {
    assert!(api()
        .retrieve_user_request(FIRST_USER_ID, WPContext::Edit)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .is_ok());
}

#[tokio::test]
async fn retrieve_user_with_embed_context() {
    assert!(api()
        .retrieve_user_request(FIRST_USER_ID, WPContext::Embed)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_embed_context)
        .is_ok());
}

#[tokio::test]
async fn retrieve_user_with_view_context() {
    assert!(api()
        .retrieve_user_request(FIRST_USER_ID, WPContext::View)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_view_context)
        .is_ok());
}

#[tokio::test]
async fn retrieve_current_user_with_edit_context() {
    assert!(api()
        .retrieve_current_user_request(WPContext::Edit)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .is_ok());
}

#[tokio::test]
async fn retrieve_current_user_with_embed_context() {
    assert!(api()
        .retrieve_current_user_request(WPContext::Embed)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_embed_context)
        .is_ok());
}

#[tokio::test]
async fn retrieve_current_user_with_view_context() {
    assert!(api()
        .retrieve_current_user_request(WPContext::View)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_view_context)
        .is_ok());
}
