use wp_api::{UserCreateParams, UserDeleteParams, UserUpdateParams};
use wp_db::{DbUser, DbUserMeta};

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
        let mut params = UserCreateParams::default();
        params.username = username.to_string();
        params.email = email.to_string();
        params.password = password.to_string();
        let created_user = api()
            .create_user_request(&params)
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
async fn update_user_name() {
    let new_name = "new_name";
    let mut params = UserUpdateParams::default();
    params.name = Some(new_name.to_string());
    test_update_user(params, |user, _| {
        assert_eq!(user.name, new_name);
    })
    .await;
}

#[tokio::test]
async fn update_user_first_name() {
    let new_first_name = "new_first_name";
    let mut params = UserUpdateParams::default();
    params.first_name = Some(new_first_name.to_string());
    test_update_user(params, |_, meta_list| {
        let db_first_name = meta_list
            .into_iter()
            .find_map(|m| {
                if m.meta_key == "first_name" {
                    Some(m.meta_value)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(db_first_name, new_first_name);
    })
    .await;
}

#[tokio::test]
async fn update_user_slug() {
    let new_slug = "new_slug";
    let mut params = UserUpdateParams::default();
    params.slug = Some(new_slug.to_string());
    test_update_user(params, |user, _| {
        assert_eq!(user.slug, new_slug);
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

        let db_user_after_update = db.fetch_db_user(FIRST_USER_ID.0 as u64).await.unwrap();
        let db_user_meta_after_update =
            db.fetch_db_user_meta(FIRST_USER_ID.0 as u64).await.unwrap();
        assert(db_user_after_update, db_user_meta_after_update);
    })
    .await;
}
