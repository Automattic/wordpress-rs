use test_helpers::SECOND_USER_EMAIL;
use wp_api::{
    UserCreateParams, UserCreateParamsBuilder, UserDeleteParams, UserId, UserListParams,
    UserUpdateParamsBuilder, WPApiHelper, WPApiParamUsersOrderBy, WPAuthentication, WPContext,
    WPRestErrorCode,
};

use crate::test_helpers::{
    api, api_as_subscriber, AssertWpError, WPNetworkRequestExecutor, WPNetworkResponseParser,
    FIRST_USER_ID, SECOND_USER_ID,
};

pub mod test_helpers;
pub mod wp_db;

#[tokio::test]
async fn list_users_with_roles_err_user_cannot_view() {
    let mut params = UserListParams::default();
    params.roles = vec!["foo".to_string()];
    api_as_subscriber()
        .list_users_request(WPContext::Edit, &Some(params))
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_list_users_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::UserCannotView);
}

#[tokio::test]
async fn list_users_with_capabilities_err_user_cannot_view() {
    let mut params = UserListParams::default();
    params.capabilities = vec!["foo".to_string()];
    api_as_subscriber()
        .list_users_request(WPContext::Edit, &Some(params))
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_list_users_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::UserCannotView);
}

#[tokio::test]
async fn list_users_err_forbidden_context() {
    api_as_subscriber()
        .list_users_request(WPContext::Edit, &None)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_list_users_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::ForbiddenContext);
}

#[tokio::test]
async fn list_users_err_forbidden_orderby_email() {
    let mut params = UserListParams::default();
    params.orderby = Some(WPApiParamUsersOrderBy::Email);
    api_as_subscriber()
        .list_users_request(WPContext::View, &Some(params))
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_list_users_response_with_view_context)
        .assert_wp_error(WPRestErrorCode::ForbiddenOrderBy);
}

#[tokio::test]
async fn list_users_orderby_registered_date_err_forbidden_orderby() {
    let mut params = UserListParams::default();
    params.orderby = Some(WPApiParamUsersOrderBy::RegisteredDate);
    api_as_subscriber()
        .list_users_request(WPContext::View, &Some(params))
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_list_users_response_with_view_context)
        .assert_wp_error(WPRestErrorCode::ForbiddenOrderBy);
}

#[tokio::test]
async fn list_users_err_forbidden_who() {
    let mut params = UserListParams::default();
    params.who = Some("authors".to_string());
    api_as_subscriber()
        .list_users_request(WPContext::View, &Some(params))
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_list_users_response_with_view_context)
        .assert_wp_error(WPRestErrorCode::ForbiddenWho);
}

#[tokio::test]
async fn retrieve_user_err_user_invalid_id() {
    api()
        .retrieve_user_request(UserId(987654321), WPContext::Edit)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::UserInvalidId);
}

#[tokio::test]
async fn retrieve_user_err_unauthorized() {
    WPApiHelper::new(
        test_helpers::test_credentials().site_url,
        WPAuthentication::None,
    )
    .retrieve_current_user_request(WPContext::Edit)
    .execute()
    .await
    .unwrap()
    .parse(wp_api::parse_retrieve_user_response_with_edit_context)
    .assert_wp_error(WPRestErrorCode::Unauthorized);
}

#[tokio::test]
async fn create_user_err_cannot_create_user() {
    api_as_subscriber()
        .create_user_request(&valid_user_create_params())
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::CannotCreateUser);
}

#[tokio::test]
async fn create_user_err_user_exists() {
    let mut request = api().create_user_request(&valid_user_create_params());
    // There is no way to create a request that'll result in `WPRestErrorCode::UserExists`
    // So, we have to manually modify the request
    request.url.push_str("?id=1");
    request
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::UserExists);
}

#[tokio::test]
async fn update_user_err_cannot_edit_roles() {
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
        .assert_wp_error(WPRestErrorCode::CannotEditRoles);
}

#[tokio::test]
async fn update_user_err_cannot_edit() {
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
        .assert_wp_error(WPRestErrorCode::CannotEdit);
}

#[tokio::test]
async fn update_user_err_invalid_param() {
    let user_update_params = UserUpdateParamsBuilder::default()
        .email(Some("not_valid".to_string()))
        .build()
        .unwrap();
    api()
        .update_user_request(FIRST_USER_ID, &user_update_params)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::InvalidParam);
}

#[tokio::test]
async fn update_user_err_user_invalid_email() {
    let user_update_params = UserUpdateParamsBuilder::default()
        .email(Some(SECOND_USER_EMAIL.to_string()))
        .build()
        .unwrap();
    // Can't update user's email to an email that's already in use
    api()
        .update_user_request(FIRST_USER_ID, &user_update_params)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::UserInvalidEmail);
}

#[tokio::test]
async fn delete_user_err_user_invalid_reassign() {
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
        .assert_wp_error(WPRestErrorCode::UserInvalidReassign);
}

#[tokio::test]
async fn delete_current_user_err_user_invalid_reassign() {
    api()
        .delete_current_user_request(&UserDeleteParams {
            reassign: UserId(987654321),
        })
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::UserInvalidReassign);
}

fn valid_user_create_params() -> UserCreateParams {
    UserCreateParamsBuilder::default()
        .username("t_username".to_string())
        .email("t_email@foo.com".to_string())
        .password("t_password".to_string())
        .build()
        .unwrap()
}
