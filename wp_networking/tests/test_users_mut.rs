use wp_api::{UserCreateParamsBuilder, UserDeleteParams, UserUpdateParams};
use wp_db::DbUser;

use crate::test_helpers::{
    api, WPNetworkRequestExecutor, WPNetworkResponseParser, FIRST_USER_ID, SECOND_USER_ID,
};

pub mod test_helpers;
pub mod wp_db;

#[tokio::test]
async fn create_user() {
    wp_db::run_and_restore(|mut db| async move {
        let username = "t_username";
        let email = "t_email@foo.com";
        let password = "t_password";

        // Create a user using the API
        let user_create_params = UserCreateParamsBuilder::default()
            .username(username.to_string())
            .email(email.to_string())
            .password(password.to_string())
            .build()
            .unwrap();
        let created_user = api()
            .create_user_request(&user_create_params)
            .execute()
            .await
            .unwrap()
            .parse(wp_api::parse_retrieve_user_response_with_edit_context)
            .unwrap();

        // Assert that the user is in DB
        let created_user_from_db = db.fetch_db_user(created_user.id.0 as u64).await.unwrap();
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
            db.fetch_db_user(SECOND_USER_ID.0 as u64).await.unwrap_err(),
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
        let user_delete_response = api()
            .delete_current_user_request(&user_delete_params)
            .execute()
            .await;
        assert!(user_delete_response.is_ok());

        // Assert that the DB doesn't have a record of the user anymore
        assert!(matches!(
            // The first user is also the current user
            db.fetch_db_user(FIRST_USER_ID.0 as u64).await.unwrap_err(),
            sqlx::Error::RowNotFound
        ));
    })
    .await;
}

#[tokio::test]
async fn update_user_slug() {
    let new_slug = "new_slug";
    let mut params = UserUpdateParams::default();
    params.slug = Some(new_slug.to_string());
    test_update_user(params, |db_user| {
        assert_eq!(db_user.slug, new_slug);
    })
    .await;
}

async fn test_update_user<F>(params: UserUpdateParams, assert: F)
where
    F: Fn(DbUser) -> (),
{
    wp_db::run_and_restore(|mut db| async move {
        let user_update_response = api()
            .update_user_request(FIRST_USER_ID, &params)
            .execute()
            .await;
        assert!(user_update_response.is_ok());

        let first_user_after_update = db.fetch_db_user(FIRST_USER_ID.0 as u64).await.unwrap();
        assert(first_user_after_update);
    })
    .await;
}
