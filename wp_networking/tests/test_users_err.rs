use wp_api::{
    UserCreateParams, UserCreateParamsBuilder, UserDeleteParams, UserId, UserListParams,
    UserUpdateParamsBuilder, WPApiParamUsersOrderBy, WPContext, WPErrorCode,
};

use crate::test_helpers::{
    api, api_as_subscriber, AssertWpError, WPNetworkRequestExecutor, WPNetworkResponseParser,
    FIRST_USER_ID, SECOND_USER_ID,
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
async fn list_users_forbidden_who() {
    let mut params = UserListParams::default();
    params.who = Some("authors".to_string());
    api_as_subscriber()
        .list_users_request(WPContext::View, &Some(params))
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_list_users_response_with_view_context)
        .assert_wp_error(WPErrorCode::ForbiddenWho);
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
async fn cannot_create_user() {
    api_as_subscriber()
        .create_user_request(&valid_user_create_params())
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPErrorCode::CannotCreateUser);
}

#[tokio::test]
async fn user_exists() {
    let mut request = api().create_user_request(&valid_user_create_params());
    // There is no way to create a request that'll result in `WPErrorCode::UserExists`
    // So, we have to manually modify the request
    request.url.push_str("?id=1");
    request
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPErrorCode::UserExists);
}

#[tokio::test]
async fn cannot_edit_roles() {
    let user_update_params = UserUpdateParamsBuilder::default()
        .roles(vec!["new_role".to_string()])
        .build()
        .unwrap();
    // Subscribers can't update their roles
    api_as_subscriber()
        .update_user_request(SECOND_USER_ID, &user_update_params)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPErrorCode::CannotEditRoles);
}

#[tokio::test]
async fn cannot_edit() {
    let user_update_params = UserUpdateParamsBuilder::default()
        .slug(Some("new_slug".to_string()))
        .build()
        .unwrap();
    // Subscribers can't update someone else's slug
    api_as_subscriber()
        .update_user_request(FIRST_USER_ID, &user_update_params)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPErrorCode::CannotEdit);
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

fn valid_user_create_params() -> UserCreateParams {
    UserCreateParamsBuilder::default()
        .username("t_username".to_string())
        .email("t_email@foo.com".to_string())
        .password("t_password".to_string())
        .build()
        .unwrap()
}
