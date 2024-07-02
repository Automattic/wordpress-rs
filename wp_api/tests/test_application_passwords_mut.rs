use integration_test_common::AssertResponse;
use wp_api::{application_passwords::ApplicationPasswordCreateParams, users::UserId};
use wp_db::DbUserMeta;

use crate::integration_test_common::{request_builder, FIRST_USER_ID};

pub mod integration_test_common;
pub mod wp_db;

#[tokio::test]
async fn create_application_password() {
    wp_db::run_and_restore(|mut db| async move {
        let password_name = "IntegrationTest";
        // Assert that the application password is not in DB
        assert!(
            !db_application_password_meta_for_user(&mut db, &FIRST_USER_ID)
                .await
                .unwrap()
                .meta_value
                .contains(password_name)
        );

        // Create an application password using the API
        let params = ApplicationPasswordCreateParams {
            app_id: None,
            name: password_name.to_string(),
        };
        let created_application_password = request_builder()
            .application_passwords()
            .create(&FIRST_USER_ID, &params)
            .await
            .assert_response();

        // Assert that the application password is in DB
        let db_user_meta_after_update =
            db_application_password_meta_for_user(&mut db, &FIRST_USER_ID).await;
        assert!(db_user_meta_after_update.is_some());
        let meta_value = db_user_meta_after_update.unwrap().meta_value;
        assert!(meta_value.contains(password_name));
        assert!(meta_value.contains(&created_application_password.uuid.uuid));
    })
    .await;
}

async fn db_application_password_meta_for_user(
    db: &mut wp_db::WordPressDb,
    user_id: &UserId,
) -> Option<DbUserMeta> {
    db.user_meta(user_id.0 as u64)
        .await
        .unwrap()
        .into_iter()
        .find(|m| m.meta_key == "_application_passwords")
}
