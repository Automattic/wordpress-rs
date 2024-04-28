use wp_api::{UserListParams, WPApiParamOrder, WPApiParamUsersOrderBy, WPContext};

use crate::test_helpers::{
    api, WPNetworkRequestExecutor, WPNetworkResponseParser, FIRST_USER_ID, SECOND_USER_ID,
};

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
async fn list_users_param_page() {
    let mut params = UserListParams::default();
    params.page = Some(2);
    test_user_list_params(params).await;
}

#[tokio::test]
async fn list_users_param_per_page() {
    let mut params = UserListParams::default();
    params.per_page = Some(2);
    test_user_list_params(params).await;
}

#[tokio::test]
async fn list_users_param_search() {
    let mut params = UserListParams::default();
    params.search = Some("foo".to_string());
    test_user_list_params(params).await;
}

#[tokio::test]
async fn list_users_param_exclude() {
    let mut params = UserListParams::default();
    params.exclude = vec![FIRST_USER_ID];
    test_user_list_params(params).await;
}

#[tokio::test]
async fn list_users_param_include() {
    let mut params = UserListParams::default();
    params.include = vec![SECOND_USER_ID];
    test_user_list_params(params).await;
}

#[tokio::test]
async fn list_users_param_offset() {
    let mut params = UserListParams::default();
    params.offset = Some(2);
    test_user_list_params(params).await;
}

#[tokio::test]
async fn list_users_param_order_asc() {
    let mut params = UserListParams::default();
    params.order = Some(WPApiParamOrder::Asc);
    test_user_list_params(params).await;
}

#[tokio::test]
async fn list_users_param_order_desc() {
    let mut params = UserListParams::default();
    params.order = Some(WPApiParamOrder::Desc);
    test_user_list_params(params).await;
}

#[tokio::test]
async fn list_users_param_orderby_id() {
    let mut params = UserListParams::default();
    params.orderby = Some(WPApiParamUsersOrderBy::Id);
    test_user_list_params(params).await;
}

#[tokio::test]
async fn list_users_param_orderby_include() {
    let mut params = UserListParams::default();
    params.orderby = Some(WPApiParamUsersOrderBy::Include);
    test_user_list_params(params).await;
}

#[tokio::test]
async fn list_users_param_orderby_name() {
    let mut params = UserListParams::default();
    params.orderby = Some(WPApiParamUsersOrderBy::Name);
    test_user_list_params(params).await;
}

#[tokio::test]
async fn list_users_param_orderby_registered_date() {
    let mut params = UserListParams::default();
    params.orderby = Some(WPApiParamUsersOrderBy::RegisteredDate);
    test_user_list_params(params).await;
}

#[tokio::test]
async fn list_users_param_orderby_slug() {
    let mut params = UserListParams::default();
    params.orderby = Some(WPApiParamUsersOrderBy::Slug);
    test_user_list_params(params).await;
}

#[tokio::test]
async fn list_users_param_orderby_include_slugs() {
    let mut params = UserListParams::default();
    params.orderby = Some(WPApiParamUsersOrderBy::IncludeSlugs);
    test_user_list_params(params).await;
}

#[tokio::test]
async fn list_users_param_orderby_email() {
    let mut params = UserListParams::default();
    params.orderby = Some(WPApiParamUsersOrderBy::Email);
    test_user_list_params(params).await;
}

#[tokio::test]
async fn list_users_param_orderby_url() {
    let mut params = UserListParams::default();
    params.orderby = Some(WPApiParamUsersOrderBy::Url);
    test_user_list_params(params).await;
}

#[tokio::test]
async fn list_users_param_slug() {
    let mut params = UserListParams::default();
    params.slug = vec!["foo".to_string()];
    test_user_list_params(params).await;
}

#[tokio::test]
async fn list_users_param_roles() {
    let mut params = UserListParams::default();
    params.roles = vec!["foo".to_string()];
    test_user_list_params(params).await;
}

#[tokio::test]
async fn list_users_param_capabilities() {
    let mut params = UserListParams::default();
    params.capabilities = vec!["foo".to_string()];
    test_user_list_params(params).await;
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

async fn test_user_list_params(params: UserListParams) {
    let parsed_response = api()
        .list_users_request(WPContext::Edit, &Some(params))
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_list_users_response_with_edit_context);
    assert!(
        parsed_response.is_ok(),
        "Response was: '{:?}'",
        parsed_response
    );
}
