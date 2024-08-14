use serial_test::serial;
use wp_api::users::{UserCreateParams, UserDeleteParams, UserUpdateParams};
use wp_api_integration_tests::{
    api_client,
    backend::{Backend, RestoreServer},
    AssertResponse, FIRST_USER_ID, SECOND_USER_ID,
};
use wp_cli::{WpCliUser, WpCliUserMeta};

#[tokio::test]
#[serial]
async fn create_user() {
    let username = "t_username";
    let email = "t_email@example.com";
    let password = "t_password";

    // Create a user using the API
    let params = UserCreateParams::new(
        username.to_string(),
        email.to_string(),
        password.to_string(),
    );
    let created_user = api_client().users().create(&params).await.assert_response();

    // Assert that the user is created
    let created_user_from_wp_cli = Backend::user(&created_user.id).await;
    assert_eq!(created_user_from_wp_cli.username, username);
    assert_eq!(created_user_from_wp_cli.email, email);

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn delete_user() {
    // Delete the user using the API and ensure it's successful
    let user_delete_params = UserDeleteParams {
        reassign: FIRST_USER_ID,
    };
    let user_delete_response = api_client()
        .users()
        .delete(&SECOND_USER_ID, &user_delete_params)
        .await;
    assert!(user_delete_response.is_ok());

    // Assert that the user was deleted
    assert!(
        !Backend::users()
            .await
            .into_iter()
            .any(|u| u.id == SECOND_USER_ID.0 as i64),
        "User wasn't deleted"
    );

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn delete_current_user() {
    // Delete the user using the API and ensure it's successful
    let user_delete_params = UserDeleteParams {
        reassign: SECOND_USER_ID,
    };
    let deleted_user = api_client()
        .users()
        .delete_me(&user_delete_params)
        .await
        .assert_response();
    assert!(deleted_user.deleted);
    assert_eq!(FIRST_USER_ID, deleted_user.previous.id);

    // Assert that the user was deleted
    assert!(
        !Backend::users()
            .await
            .into_iter()
            .any(|u| u.id == FIRST_USER_ID.0 as i64),
        "User wasn't deleted"
    );

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn update_user_name() {
    let new_name = "new_name";
    let params = UserUpdateParams {
        name: Some(new_name.to_string()),
        ..Default::default()
    };
    test_update_user(params, |user, _| {
        assert_eq!(user.name, new_name);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn update_user_first_name() {
    let new_first_name = "new_first_name";
    let params = UserUpdateParams {
        first_name: Some(new_first_name.to_string()),
        ..Default::default()
    };
    test_update_user(params, |_, meta_list| {
        assert_eq!(find_meta(&meta_list, "first_name"), new_first_name);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn update_user_last_name() {
    let new_last_name = "new_last_name";
    let params = UserUpdateParams {
        last_name: Some(new_last_name.to_string()),
        ..Default::default()
    };
    test_update_user(params, |_, meta_list| {
        assert_eq!(find_meta(&meta_list, "last_name"), new_last_name);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn update_user_email() {
    let new_email = "new_email@example.com";
    let params = UserUpdateParams {
        email: Some(new_email.to_string()),
        ..Default::default()
    };
    test_update_user(params, |user, _| {
        assert_eq!(user.email, new_email);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn update_user_url() {
    let new_url = "https://new_url";
    let params = UserUpdateParams {
        url: Some(new_url.to_string()),
        ..Default::default()
    };
    test_update_user(params, |user, _| {
        assert_eq!(user.url, new_url);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn update_user_description() {
    let new_description = "new_description";
    let params = UserUpdateParams {
        description: Some(new_description.to_string()),
        ..Default::default()
    };
    test_update_user(params, |_, meta_list| {
        assert_eq!(find_meta(&meta_list, "description"), new_description);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn update_user_nickname() {
    let new_nickname = "new_nickname";
    let params = UserUpdateParams {
        nickname: Some(new_nickname.to_string()),
        ..Default::default()
    };
    test_update_user(params, |_, meta_list| {
        assert_eq!(find_meta(&meta_list, "nickname"), new_nickname);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn update_user_slug() {
    let new_slug = "new_slug";
    let params = UserUpdateParams {
        slug: Some(new_slug.to_string()),
        ..Default::default()
    };
    test_update_user(params, |user, _| {
        assert_eq!(
            user.url_slug,
            Some(format!("http://localhost/author/{}/", new_slug))
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn update_user_roles() {
    let new_role = "author";
    let params = UserUpdateParams {
        roles: vec![new_role.to_string()],
        ..Default::default()
    };

    api_client()
        .users()
        .update(&SECOND_USER_ID, &params)
        .await
        .assert_response();

    let updated_user = Backend::users()
        .await
        .into_iter()
        .find(|u| u.id == SECOND_USER_ID.0 as i64)
        .expect("Failed to find the updated user");
    assert_eq!(updated_user.roles, new_role);

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn update_user_password() {
    let new_password = "new_password";
    let params = UserUpdateParams {
        password: Some(new_password.to_string()),
        ..Default::default()
    };
    // It's quite tricky to validate the password change, so we just ensure the request was
    // successful
    api_client()
        .users()
        .update(&FIRST_USER_ID, &params)
        .await
        .assert_response();

    RestoreServer::db().await;
}

async fn test_update_user<F>(params: UserUpdateParams, assert: F)
where
    F: Fn(WpCliUser, Vec<WpCliUserMeta>),
{
    api_client()
        .users()
        .update(&FIRST_USER_ID, &params)
        .await
        .assert_response();

    let updated_user = Backend::user(&FIRST_USER_ID).await;
    let updated_user_meta = Backend::user_meta(&FIRST_USER_ID).await;
    assert(updated_user, updated_user_meta);

    RestoreServer::db().await;
}

fn find_meta(meta_list: &[WpCliUserMeta], meta_key: &str) -> String {
    meta_list
        .iter()
        .find_map(|m| {
            if m.meta_key == meta_key {
                Some(m.meta_value.clone())
            } else {
                None
            }
        })
        .unwrap()
}
