use test_helpers::FIRST_USER_ID;
use wp_api::{UserListParams, WPContext};

pub mod test_helpers;

#[tokio::test]
async fn list_users_with_edit_context() {
    assert!(test_helpers::list_users(WPContext::Edit, None, |r| {
        wp_api::parse_list_users_response_with_edit_context(&r)
    })
    .await
    .is_ok());
}

#[tokio::test]
async fn list_users_with_embed_context() {
    assert!(test_helpers::list_users(WPContext::Embed, None, |r| {
        wp_api::parse_list_users_response_with_embed_context(&r)
    })
    .await
    .is_ok());
}

#[tokio::test]
async fn list_users_with_view_context() {
    assert!(test_helpers::list_users(WPContext::View, None, |r| {
        wp_api::parse_list_users_response_with_view_context(&r)
    })
    .await
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
    assert!(
        test_helpers::list_users(WPContext::Edit, Some(params), |r| {
            wp_api::parse_list_users_response_with_edit_context(&r)
        })
        .await
        .is_ok()
    );
}

#[tokio::test]
async fn retrieve_user_with_edit_context() {
    assert!(
        test_helpers::retrieve_user(FIRST_USER_ID, WPContext::Edit, |r| {
            wp_api::parse_retrieve_user_response_with_edit_context(&r)
        })
        .await
        .is_ok()
    );
}

#[tokio::test]
async fn retrieve_user_with_embed_context() {
    assert!(
        test_helpers::retrieve_user(FIRST_USER_ID, WPContext::Embed, |r| {
            wp_api::parse_retrieve_user_response_with_embed_context(&r)
        })
        .await
        .is_ok()
    );
}

#[tokio::test]
async fn retrieve_user_with_view_context() {
    assert!(test_helpers::retrieve_me(WPContext::View, |r| {
        wp_api::parse_retrieve_user_response_with_view_context(&r)
    })
    .await
    .is_ok());
}

#[tokio::test]
async fn retrieve_me_with_edit_context() {
    assert!(test_helpers::retrieve_me(WPContext::Edit, |r| {
        wp_api::parse_retrieve_user_response_with_edit_context(&r)
    })
    .await
    .is_ok());
}

#[tokio::test]
async fn retrieve_me_with_embed_context() {
    assert!(test_helpers::retrieve_me(WPContext::Embed, |r| {
        wp_api::parse_retrieve_user_response_with_embed_context(&r)
    })
    .await
    .is_ok());
}

#[tokio::test]
async fn retrieve_me_with_view_context() {
    assert!(test_helpers::retrieve_me(WPContext::View, |r| {
        wp_api::parse_retrieve_user_response_with_view_context(&r)
    })
    .await
    .is_ok());
}
