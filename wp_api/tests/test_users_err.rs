use integration_test_common::SECOND_USER_EMAIL;
use wp_api::{
    request::endpoint::WpEndpointUrl,
    users::{
        UserCreateParams, UserDeleteParams, UserId, UserListParams, UserUpdateParams,
        WPApiParamUsersOrderBy, WPApiParamUsersWho,
    },
    WPAuthentication, WPRestErrorCode, WpContext,
};

use crate::integration_test_common::{
    request_builder, request_builder_as_subscriber, AssertWpError, WpNetworkRequestExecutor,
    FIRST_USER_ID, SECOND_USER_ID, SECOND_USER_SLUG,
};

pub mod integration_test_common;

#[tokio::test]
async fn create_user_err_cannot_create_user() {
    request_builder_as_subscriber()
        .users()
        .create(&valid_user_create_params())
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::CannotCreateUser);
}

#[tokio::test]
async fn delete_user_err_user_cannot_delete() {
    request_builder_as_subscriber()
        .users()
        .delete(
            FIRST_USER_ID,
            &UserDeleteParams {
                reassign: SECOND_USER_ID,
            },
        )
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::UserCannotDelete);
}

#[tokio::test]
async fn delete_user_err_user_invalid_reassign() {
    request_builder()
        .users()
        .delete(
            FIRST_USER_ID,
            &UserDeleteParams {
                reassign: UserId(987654321),
            },
        )
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::UserInvalidReassign);
}

#[tokio::test]
async fn delete_current_user_err_user_invalid_reassign() {
    request_builder()
        .users()
        .delete_me(&UserDeleteParams {
            reassign: UserId(987654321),
        })
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::UserInvalidReassign);
}

#[tokio::test]
async fn list_users_err_forbidden_context() {
    request_builder_as_subscriber()
        .users()
        .list(WpContext::Edit, &None)
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_list_users_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::ForbiddenContext);
}

#[tokio::test]
async fn list_users_err_forbidden_orderby_email() {
    let params = UserListParams {
        orderby: Some(WPApiParamUsersOrderBy::Email),
        ..Default::default()
    };
    request_builder_as_subscriber()
        .users()
        .list(WpContext::View, &Some(params))
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_list_users_response_with_view_context)
        .assert_wp_error(WPRestErrorCode::ForbiddenOrderBy);
}

#[tokio::test]
async fn list_users_err_forbidden_who() {
    let params = UserListParams {
        who: Some(WPApiParamUsersWho::Authors),
        ..Default::default()
    };
    request_builder_as_subscriber()
        .users()
        .list(WpContext::View, &Some(params))
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_list_users_response_with_view_context)
        .assert_wp_error(WPRestErrorCode::ForbiddenWho);
}

#[tokio::test]
async fn list_users_with_capabilities_err_user_cannot_view() {
    let params = UserListParams {
        capabilities: vec!["foo".to_string()],
        ..Default::default()
    };
    request_builder_as_subscriber()
        .users()
        .list(WpContext::Edit, &Some(params))
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_list_users_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::UserCannotView);
}

#[tokio::test]
async fn list_users_with_roles_err_user_cannot_view() {
    let params = UserListParams {
        roles: vec!["foo".to_string()],
        ..Default::default()
    };
    request_builder_as_subscriber()
        .users()
        .list(WpContext::Edit, &Some(params))
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_list_users_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::UserCannotView);
}

#[tokio::test]
async fn list_users_orderby_registered_date_err_forbidden_orderby() {
    let params = UserListParams {
        orderby: Some(WPApiParamUsersOrderBy::RegisteredDate),
        ..Default::default()
    };
    request_builder_as_subscriber()
        .users()
        .list(WpContext::View, &Some(params))
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_list_users_response_with_view_context)
        .assert_wp_error(WPRestErrorCode::ForbiddenOrderBy);
}

#[tokio::test]
async fn retrieve_user_err_user_invalid_id() {
    request_builder()
        .users()
        .retrieve(UserId(987654321), WpContext::Edit)
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::UserInvalidId);
}

#[tokio::test]
async fn retrieve_user_err_unauthorized() {
    wp_api::WpRequestBuilder::new(
        integration_test_common::read_test_credentials_from_file().site_url,
        WPAuthentication::None,
    )
    .expect("Site url is generated by our tooling")
    .users()
    .retrieve_me(WpContext::Edit)
    .execute()
    .await
    .unwrap()
    .parse_with(wp_api::users::parse_retrieve_user_response_with_edit_context)
    .assert_wp_error(WPRestErrorCode::Unauthorized);
}

#[tokio::test]
async fn update_user_err_cannot_edit() {
    let params = UserUpdateParams {
        slug: Some("new_slug".to_string()),
        ..Default::default()
    };
    // Subscribers can't update someone else's slug
    request_builder_as_subscriber()
        .users()
        .update(FIRST_USER_ID, &params)
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::CannotEdit);
}

#[tokio::test]
async fn update_user_err_user_invalid_argument() {
    let mut request = request_builder()
        .users()
        .update(FIRST_USER_ID, &UserUpdateParams::default());
    request.body = Some(
        serde_json::json!({
            "username": "new_username",
        })
        .to_string()
        .into_bytes(),
    );
    // Usernames are not editable
    request
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::UserInvalidArgument);
}

#[tokio::test]
async fn update_user_err_cannot_edit_roles() {
    let params = UserUpdateParams {
        roles: vec!["new_role".to_string()],
        ..Default::default()
    };
    // Subscribers can't update their roles
    request_builder_as_subscriber()
        .users()
        .update(SECOND_USER_ID, &params)
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::CannotEditRoles);
}

#[tokio::test]
async fn update_user_err_user_invalid_email() {
    let params = UserUpdateParams {
        email: Some(SECOND_USER_EMAIL.to_string()),
        ..Default::default()
    };
    // Can't update user's email to an email that's already in use
    request_builder()
        .users()
        .update(FIRST_USER_ID, &params)
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::UserInvalidEmail);
}

#[tokio::test]
async fn update_user_email_err_invalid_param() {
    let params = UserUpdateParams {
        email: Some("not_valid".to_string()),
        ..Default::default()
    };
    request_builder()
        .users()
        .update(FIRST_USER_ID, &params)
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::InvalidParam);
}

#[tokio::test]
async fn update_user_password_err_invalid_param() {
    let params = UserUpdateParams {
        password: Some("".to_string()),
        ..Default::default()
    };
    request_builder()
        .users()
        .update(FIRST_USER_ID, &params)
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::InvalidParam);
}

#[tokio::test]
async fn update_user_err_user_invalid_role() {
    let params = UserUpdateParams {
        roles: vec!["doesnt_exist".to_string()],
        ..Default::default()
    };
    // Can't update user's email to a role that doesn't exist
    request_builder()
        .users()
        .update(FIRST_USER_ID, &params)
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::UserInvalidRole);
}

#[tokio::test]
async fn update_user_err_user_invalid_slug() {
    let params = UserUpdateParams {
        slug: Some(SECOND_USER_SLUG.to_string()),
        ..Default::default()
    };
    // Can't update user's slug to a slug that's already in use
    request_builder()
        .users()
        .update(FIRST_USER_ID, &params)
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::UserInvalidSlug);
}

// In the following tests, we manually modify the request because the errors they test are not
// possible to get without it. We believe these tests are still useful as they act as a
// documentation for how these errors might be received.

#[tokio::test]
async fn create_user_err_user_exists() {
    let mut request = request_builder()
        .users()
        .create(&valid_user_create_params());
    request.url.0.push_str("?id=1");
    request
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::UserExists);
}

#[tokio::test]
async fn delete_user_err_trash_not_supported() {
    let mut request = request_builder().users().delete(
        FIRST_USER_ID,
        &UserDeleteParams {
            reassign: SECOND_USER_ID,
        },
    );
    request.url = WpEndpointUrl(request.url.0.replace("&force=true", ""));
    request
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::TrashNotSupported);
}

// Helpers

fn valid_user_create_params() -> UserCreateParams {
    UserCreateParams::new(
        "t_username".to_string(),
        "t_email@example.com".to_string(),
        "t_password".to_string(),
    )
}
