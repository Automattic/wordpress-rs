use wp_api::users::{UserCreateParams, UserDeleteParams, UserUpdateParams};
use wp_db::{DbUser, DbUserMeta};

use crate::integration_test_common::{
    api, WPNetworkRequestExecutor, WPNetworkResponseParser, FIRST_USER_ID, SECOND_USER_ID,
};

pub mod integration_test_common;
pub mod wp_db;

#[tokio::test]
async fn create_user() {
    wp_db::run_and_restore(|mut db| async move {
        let username = "t_username";
        let email = "t_email@example.com";
        let password = "t_password";

        // Create a user using the API
        let params = UserCreateParams::new(
            username.to_string(),
            email.to_string(),
            password.to_string(),
        );
        let created_user = api()
            .create_user_request(&params)
            .execute()
            .await
            .unwrap()
            .parse(wp_api::users::parse_retrieve_user_response_with_edit_context)
            .unwrap();

        // Assert that the user is in DB
        let created_user_from_db = db.user(created_user.id.0 as u64).await.unwrap();
        assert_eq!(created_user_from_db.username, username);
        assert_eq!(created_user_from_db.email, email);
    })
    .await;
}

#[tokio::test]
async fn delete_user() {
    wp_db::run_and_restore(|mut db| async move {
        // Delete the user using the API and ensure it's successful
        let user_delete_params = UserDeleteParams {
            reassign: FIRST_USER_ID,
        };
        let user_delete_response = api()
            .delete_user_request(SECOND_USER_ID, &user_delete_params)
            .execute()
            .await;
        assert!(user_delete_response.is_ok());

        // Assert that the DB doesn't have a record of the user anymore
        assert!(matches!(
            db.user(SECOND_USER_ID.0 as u64).await.unwrap_err(),
            sqlx::Error::RowNotFound
        ));
    })
    .await;
}

#[tokio::test]
async fn delete_current_user() {
    wp_db::run_and_restore(|mut db| async move {
        // Delete the user using the API and ensure it's successful
        let user_delete_params = UserDeleteParams {
            reassign: SECOND_USER_ID,
        };
        let deleted_user = api()
            .delete_current_user_request(&user_delete_params)
            .execute()
            .await
            .unwrap()
            .parse(wp_api::users::parse_delete_user_response)
            .unwrap();
        assert_eq!(true, deleted_user.deleted);
        assert_eq!(FIRST_USER_ID, deleted_user.previous.id);

        // Assert that the DB doesn't have a record of the user anymore
        assert!(matches!(
            // The first user is also the current user
            db.user(FIRST_USER_ID.0 as u64).await.unwrap_err(),
            sqlx::Error::RowNotFound
        ));
    })
    .await;
}

#[tokio::test]
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
async fn update_user_slug() {
    let new_slug = "new_slug";
    let params = UserUpdateParams {
        slug: Some(new_slug.to_string()),
        ..Default::default()
    };
    test_update_user(params, |user, _| {
        assert_eq!(user.slug, new_slug);
    })
    .await;
}

#[tokio::test]
async fn update_user_roles() {
    wp_db::run_and_restore(|_| async move {
        let new_role = "new_role";
        let params = UserUpdateParams {
            roles: vec![new_role.to_string()],
            ..Default::default()
        };
        let user_update_response = api()
            .update_user_request(FIRST_USER_ID, &params)
            .execute()
            .await;
        // It's quite tricky to validate the roles from DB, so we just ensure the request was
        // successful
        assert!(user_update_response.is_ok());
    })
    .await;
}

#[tokio::test]
async fn update_user_password() {
    wp_db::run_and_restore(|_| async move {
        let new_password = "new_password";
        let params = UserUpdateParams {
            password: Some(new_password.to_string()),
            ..Default::default()
        };
        let user_update_response = api()
            .update_user_request(FIRST_USER_ID, &params)
            .execute()
            .await;
        // It's quite tricky to validate the password from DB, so we just ensure the request was
        // successful
        assert!(user_update_response.is_ok());
    })
    .await;
}

async fn test_update_user<F>(params: UserUpdateParams, assert: F)
where
    F: Fn(DbUser, Vec<DbUserMeta>) -> (),
{
    wp_db::run_and_restore(|mut db| async move {
        let user_update_response = api()
            .update_user_request(FIRST_USER_ID, &params)
            .execute()
            .await;
        assert!(user_update_response.is_ok());

        let db_user_after_update = db.user(FIRST_USER_ID.0 as u64).await.unwrap();
        let db_user_meta_after_update = db.user_meta(FIRST_USER_ID.0 as u64).await.unwrap();
        assert(db_user_after_update, db_user_meta_after_update);
    })
    .await;
}

fn find_meta(meta_list: &Vec<DbUserMeta>, meta_key: &str) -> String {
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
