use std::sync::Arc;

use integration_test_common::{AsyncWpNetworking, SECOND_USER_EMAIL};
use wp_api::{
    users::{
        UserCreateParams, UserDeleteParams, UserId, UserListParams, UserUpdateParams,
        WpApiParamUsersHasPublishedPosts, WpApiParamUsersOrderBy, WpApiParamUsersWho,
    },
    WpAuthentication, WpRestErrorCode,
};

use crate::integration_test_common::{
    request_builder, request_builder_as_subscriber, AssertWpError, FIRST_USER_ID, SECOND_USER_ID,
    SECOND_USER_SLUG,
};

pub mod integration_test_common;

#[tokio::test]
async fn create_user_err_cannot_create_user() {
    request_builder_as_subscriber()
        .users()
        .create(&valid_user_create_params())
        .await
        .assert_wp_error(WpRestErrorCode::CannotCreateUser);
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
        .await
        .assert_wp_error(WpRestErrorCode::UserCannotDelete);
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
        .await
        .assert_wp_error(WpRestErrorCode::UserInvalidReassign);
}

#[tokio::test]
async fn delete_current_user_err_user_invalid_reassign() {
    request_builder()
        .users()
        .delete_me(&UserDeleteParams {
            reassign: UserId(987654321),
        })
        .await
        .assert_wp_error(WpRestErrorCode::UserInvalidReassign);
}

#[tokio::test]
async fn list_users_err_forbidden_context() {
    request_builder_as_subscriber()
        .users()
        .list_with_edit_context(&UserListParams::default())
        .await
        .assert_wp_error(WpRestErrorCode::ForbiddenContext);
}

#[tokio::test]
async fn list_users_err_forbidden_orderby_email() {
    let params = UserListParams {
        orderby: Some(WpApiParamUsersOrderBy::Email),
        ..Default::default()
    };
    request_builder_as_subscriber()
        .users()
        .list_with_view_context(&params)
        .await
        .assert_wp_error(WpRestErrorCode::ForbiddenOrderBy);
}

#[tokio::test]
async fn list_users_err_forbidden_who() {
    let params = UserListParams {
        who: Some(WpApiParamUsersWho::Authors),
        ..Default::default()
    };
    request_builder_as_subscriber()
        .users()
        .list_with_view_context(&params)
        .await
        .assert_wp_error(WpRestErrorCode::ForbiddenWho);
}

#[tokio::test]
async fn list_users_with_capabilities_err_user_cannot_view() {
    let params = UserListParams {
        capabilities: vec!["foo".to_string()],
        ..Default::default()
    };
    request_builder_as_subscriber()
        .users()
        .list_with_edit_context(&params)
        .await
        .assert_wp_error(WpRestErrorCode::UserCannotView);
}

#[tokio::test]
async fn list_users_with_roles_err_user_cannot_view() {
    let params = UserListParams {
        roles: vec!["foo".to_string()],
        ..Default::default()
    };
    request_builder_as_subscriber()
        .users()
        .list_with_edit_context(&params)
        .await
        .assert_wp_error(WpRestErrorCode::UserCannotView);
}

#[tokio::test]
async fn list_users_orderby_registered_date_err_forbidden_orderby() {
    let params = UserListParams {
        orderby: Some(WpApiParamUsersOrderBy::RegisteredDate),
        ..Default::default()
    };
    request_builder_as_subscriber()
        .users()
        .list_with_view_context(&params)
        .await
        .assert_wp_error(WpRestErrorCode::ForbiddenOrderBy);
}

#[tokio::test]
async fn list_users_has_published_posts_err_invalid_param() {
    request_builder()
        .users()
        .list_with_edit_context(&UserListParams {
            has_published_posts: Some(WpApiParamUsersHasPublishedPosts::PostTypes(vec![
                "foo".to_string()
            ])),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpRestErrorCode::InvalidParam);
}

#[tokio::test]
async fn retrieve_user_err_user_invalid_id() {
    request_builder()
        .users()
        .retrieve_with_edit_context(UserId(987654321))
        .await
        .assert_wp_error(WpRestErrorCode::UserInvalidId);
}

#[tokio::test]
async fn retrieve_user_err_unauthorized() {
    wp_api::WpRequestBuilder::new(
        integration_test_common::read_test_credentials_from_file().site_url,
        WpAuthentication::None,
        Arc::new(AsyncWpNetworking::default()),
    )
    .users()
    .retrieve_me_with_edit_context()
    .await
    .assert_wp_error(WpRestErrorCode::Unauthorized);
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
        .await
        .assert_wp_error(WpRestErrorCode::CannotEdit);
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
        .await
        .assert_wp_error(WpRestErrorCode::CannotEditRoles);
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
        .await
        .assert_wp_error(WpRestErrorCode::UserInvalidEmail);
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
        .await
        .assert_wp_error(WpRestErrorCode::InvalidParam);
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
        .await
        .assert_wp_error(WpRestErrorCode::InvalidParam);
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
        .await
        .assert_wp_error(WpRestErrorCode::UserInvalidRole);
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
        .await
        .assert_wp_error(WpRestErrorCode::UserInvalidSlug);
}

// Helpers

fn valid_user_create_params() -> UserCreateParams {
    UserCreateParams::new(
        "t_username".to_string(),
        "t_email@example.com".to_string(),
        "t_password".to_string(),
    )
}
