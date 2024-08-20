use serial_test::parallel;
use wp_api::{
    users::{
        UserCreateParams, UserDeleteParams, UserId, UserListParams, UserUpdateParams,
        WpApiParamUsersHasPublishedPosts, WpApiParamUsersOrderBy, WpApiParamUsersWho,
    },
    WpErrorCode,
};
use wp_api_integration_tests::{
    api_client, api_client_as_subscriber, api_client_as_unauthenticated, AssertWpError,
    FIRST_USER_ID, SECOND_USER_EMAIL, SECOND_USER_ID, SECOND_USER_SLUG,
};

#[tokio::test]
#[parallel]
async fn create_user_err_cannot_create_user() {
    api_client_as_subscriber()
        .users()
        .create(&valid_user_create_params())
        .await
        .assert_wp_error(WpErrorCode::CannotCreateUser);
}

#[tokio::test]
#[parallel]
async fn delete_user_err_user_cannot_delete() {
    api_client_as_subscriber()
        .users()
        .delete(
            &FIRST_USER_ID,
            &UserDeleteParams {
                reassign: SECOND_USER_ID,
            },
        )
        .await
        .assert_wp_error(WpErrorCode::UserCannotDelete);
}

#[tokio::test]
#[parallel]
async fn delete_user_err_user_invalid_reassign() {
    api_client()
        .users()
        .delete(
            &FIRST_USER_ID,
            &UserDeleteParams {
                reassign: UserId(987654321),
            },
        )
        .await
        .assert_wp_error(WpErrorCode::UserInvalidReassign);
}

#[tokio::test]
#[parallel]
async fn delete_current_user_err_user_invalid_reassign() {
    api_client()
        .users()
        .delete_me(&UserDeleteParams {
            reassign: UserId(987654321),
        })
        .await
        .assert_wp_error(WpErrorCode::UserInvalidReassign);
}

#[tokio::test]
#[parallel]
async fn list_users_err_forbidden_context() {
    api_client_as_subscriber()
        .users()
        .list_with_edit_context(&UserListParams::default())
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

#[tokio::test]
#[parallel]
async fn list_users_err_forbidden_orderby_email() {
    let params = UserListParams {
        orderby: Some(WpApiParamUsersOrderBy::Email),
        ..Default::default()
    };
    api_client_as_subscriber()
        .users()
        .list_with_view_context(&params)
        .await
        .assert_wp_error(WpErrorCode::ForbiddenOrderBy);
}

#[tokio::test]
#[parallel]
async fn list_users_err_forbidden_who() {
    let params = UserListParams {
        who: Some(WpApiParamUsersWho::Authors),
        ..Default::default()
    };
    api_client_as_subscriber()
        .users()
        .list_with_view_context(&params)
        .await
        .assert_wp_error(WpErrorCode::ForbiddenWho);
}

#[tokio::test]
#[parallel]
async fn list_users_with_capabilities_err_user_cannot_view() {
    let params = UserListParams {
        capabilities: vec!["foo".to_string()],
        ..Default::default()
    };
    api_client_as_subscriber()
        .users()
        .list_with_edit_context(&params)
        .await
        .assert_wp_error(WpErrorCode::UserCannotView);
}

#[tokio::test]
#[parallel]
async fn list_users_with_roles_err_user_cannot_view() {
    let params = UserListParams {
        roles: vec!["foo".to_string()],
        ..Default::default()
    };
    api_client_as_subscriber()
        .users()
        .list_with_edit_context(&params)
        .await
        .assert_wp_error(WpErrorCode::UserCannotView);
}

#[tokio::test]
#[parallel]
async fn list_users_orderby_registered_date_err_forbidden_orderby() {
    let params = UserListParams {
        orderby: Some(WpApiParamUsersOrderBy::RegisteredDate),
        ..Default::default()
    };
    api_client_as_subscriber()
        .users()
        .list_with_view_context(&params)
        .await
        .assert_wp_error(WpErrorCode::ForbiddenOrderBy);
}

#[tokio::test]
#[parallel]
async fn list_users_has_published_posts_err_invalid_param() {
    api_client()
        .users()
        .list_with_edit_context(&UserListParams {
            has_published_posts: Some(WpApiParamUsersHasPublishedPosts::PostTypes(vec![
                "foo".to_string()
            ])),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::InvalidParam);
}

#[tokio::test]
#[parallel]
async fn retrieve_user_err_user_invalid_id() {
    api_client()
        .users()
        .retrieve_with_edit_context(&UserId(987654321))
        .await
        .assert_wp_error(WpErrorCode::UserInvalidId);
}

#[tokio::test]
#[parallel]
async fn retrieve_user_err_unauthorized() {
    api_client_as_unauthenticated()
        .users()
        .retrieve_me_with_edit_context()
        .await
        .assert_wp_error(WpErrorCode::Unauthorized);
}

#[tokio::test]
#[parallel]
async fn update_user_err_cannot_edit() {
    let params = UserUpdateParams {
        slug: Some("new_slug".to_string()),
        ..Default::default()
    };
    // Subscribers can't update someone else's slug
    api_client_as_subscriber()
        .users()
        .update(&FIRST_USER_ID, &params)
        .await
        .assert_wp_error(WpErrorCode::CannotEdit);
}

#[tokio::test]
#[parallel]
async fn update_user_err_cannot_edit_roles() {
    let params = UserUpdateParams {
        roles: vec!["new_role".to_string()],
        ..Default::default()
    };
    // Subscribers can't update their roles
    api_client_as_subscriber()
        .users()
        .update(&SECOND_USER_ID, &params)
        .await
        .assert_wp_error(WpErrorCode::CannotEditRoles);
}

#[tokio::test]
#[parallel]
async fn update_user_err_user_invalid_email() {
    let params = UserUpdateParams {
        email: Some(SECOND_USER_EMAIL.to_string()),
        ..Default::default()
    };
    // Can't update user's email to an email that's already in use
    api_client()
        .users()
        .update(&FIRST_USER_ID, &params)
        .await
        .assert_wp_error(WpErrorCode::UserInvalidEmail);
}

#[tokio::test]
#[parallel]
async fn update_user_email_err_invalid_param() {
    let params = UserUpdateParams {
        email: Some("not_valid".to_string()),
        ..Default::default()
    };
    api_client()
        .users()
        .update(&FIRST_USER_ID, &params)
        .await
        .assert_wp_error(WpErrorCode::InvalidParam);
}

#[tokio::test]
#[parallel]
async fn update_user_password_err_invalid_param() {
    let params = UserUpdateParams {
        password: Some("".to_string()),
        ..Default::default()
    };
    api_client()
        .users()
        .update(&FIRST_USER_ID, &params)
        .await
        .assert_wp_error(WpErrorCode::InvalidParam);
}

#[tokio::test]
#[parallel]
async fn update_user_err_user_invalid_role() {
    let params = UserUpdateParams {
        roles: vec!["doesnt_exist".to_string()],
        ..Default::default()
    };
    // Can't update user's email to a role that doesn't exist
    api_client()
        .users()
        .update(&FIRST_USER_ID, &params)
        .await
        .assert_wp_error(WpErrorCode::UserInvalidRole);
}

#[tokio::test]
#[parallel]
async fn update_user_err_user_invalid_slug() {
    let params = UserUpdateParams {
        slug: Some(SECOND_USER_SLUG.to_string()),
        ..Default::default()
    };
    // Can't update user's slug to a slug that's already in use
    api_client()
        .users()
        .update(&FIRST_USER_ID, &params)
        .await
        .assert_wp_error(WpErrorCode::UserInvalidSlug);
}

// Helpers

fn valid_user_create_params() -> UserCreateParams {
    UserCreateParams::new(
        "t_username".to_string(),
        "t_email@example.com".to_string(),
        "t_password".to_string(),
    )
}
