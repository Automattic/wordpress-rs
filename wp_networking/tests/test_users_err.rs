use wp_api::{
    UserDeleteParams, UserId, UserListParams, WPApiParamUsersOrderBy, WPContext, WPErrorCode,
};

use crate::test_helpers::{
    api, api_as_subscriber, AssertWpError, WPNetworkRequestExecutor, WPNetworkResponseParser,
    FIRST_USER_ID,
};

pub mod test_helpers;
pub mod wp_db;

#[tokio::test]
async fn list_users_with_roles_user_cannot_view() {
    let mut params = UserListParams::default();
    params.roles = vec!["foo".to_string()];
    api_as_subscriber()
        .list_users_request(WPContext::Edit, &Some(params))
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_list_users_response_with_edit_context)
        .assert_wp_error(WPErrorCode::UserCannotView);
}

#[tokio::test]
async fn list_users_with_capabilities_user_cannot_view() {
    let mut params = UserListParams::default();
    params.capabilities = vec!["foo".to_string()];
    api_as_subscriber()
        .list_users_request(WPContext::Edit, &Some(params))
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_list_users_response_with_edit_context)
        .assert_wp_error(WPErrorCode::UserCannotView);
}

#[tokio::test]
async fn list_users_forbidden_context() {
    api_as_subscriber()
        .list_users_request(WPContext::Edit, &None)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_list_users_response_with_edit_context)
        .assert_wp_error(WPErrorCode::ForbiddenContext);
}

#[tokio::test]
async fn list_users_forbidden_orderby_email() {
    let mut params = UserListParams::default();
    params.orderby = Some(WPApiParamUsersOrderBy::Email);
    api_as_subscriber()
        .list_users_request(WPContext::View, &Some(params))
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_list_users_response_with_view_context)
        .assert_wp_error(WPErrorCode::ForbiddenOrderBy);
}

#[tokio::test]
async fn list_users_forbidden_order_by_registered_date() {
    let mut params = UserListParams::default();
    params.orderby = Some(WPApiParamUsersOrderBy::RegisteredDate);
    api_as_subscriber()
        .list_users_request(WPContext::View, &Some(params))
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_list_users_response_with_view_context)
        .assert_wp_error(WPErrorCode::ForbiddenOrderBy);
}

#[tokio::test]
async fn retrieve_user_invalid_user_id() {
    api()
        .retrieve_user_request(UserId(987654321), WPContext::Edit)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPErrorCode::UserInvalidId);
}

#[tokio::test]
async fn delete_user_invalid_reassign() {
    api()
        .delete_user_request(
            FIRST_USER_ID,
            &UserDeleteParams {
                reassign: UserId(987654321),
            },
        )
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPErrorCode::UserInvalidReassign);
}

#[tokio::test]
async fn delete_current_user_invalid_reassign() {
    api()
        .delete_current_user_request(&UserDeleteParams {
            reassign: UserId(987654321),
        })
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPErrorCode::UserInvalidReassign);
}
