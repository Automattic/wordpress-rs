use serial_test::serial;
use wp_api::{
    application_passwords::{
        ApplicationPasswordCreateParams, ApplicationPasswordUpdateParams, ApplicationPasswordUuid,
    },
    users::UserId,
};
use wp_api_integration_tests::wp_db::{self, DbUserMeta};
use wp_api_integration_tests::{
    api_client, AssertResponse, FIRST_USER_ID, SECOND_USER_ID,
    TEST_CREDENTIALS_ADMIN_PASSWORD_UUID, TEST_CREDENTIALS_SUBSCRIBER_PASSWORD_UUID,
};

#[tokio::test]
#[serial]
async fn create_application_password() {
    wp_db::run_and_restore(|mut db| async move {
        let password_name = "IntegrationTest";
        // Assert that the application password name is not in DB
        assert!(
            !db_application_password_meta_for_user(&mut db, &SECOND_USER_ID)
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
        let created_application_password = api_client()
            .application_passwords()
            .create(&SECOND_USER_ID, &params)
            .await
            .assert_response();

        // Assert that the application password is in DB
        let db_user_meta_after_update =
            db_application_password_meta_for_user(&mut db, &SECOND_USER_ID).await;
        assert!(db_user_meta_after_update.is_some());
        let meta_value = db_user_meta_after_update.unwrap().meta_value;
        assert!(meta_value.contains(password_name));
        assert!(meta_value.contains(&created_application_password.uuid.uuid));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn update_application_password() {
    wp_db::run_and_restore(|mut db| async move {
        let password_name = "IntegrationTest";
        // Assert that the application password name is not in DB
        assert!(
            !db_application_password_meta_for_user(&mut db, &FIRST_USER_ID)
                .await
                .unwrap()
                .meta_value
                .contains(password_name)
        );

        // Update the application password to use the new name using the API
        let params = ApplicationPasswordUpdateParams {
            app_id: None,
            name: password_name.to_string(),
        };
        let created_application_password = api_client()
            .application_passwords()
            .update(
                &FIRST_USER_ID,
                &ApplicationPasswordUuid {
                    uuid: TEST_CREDENTIALS_ADMIN_PASSWORD_UUID.to_string(),
                },
                &params,
            )
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

#[tokio::test]
#[serial]
async fn delete_single_application_password() {
    wp_db::run_and_restore(|mut db| async move {
        let uuid = ApplicationPasswordUuid {
            uuid: TEST_CREDENTIALS_SUBSCRIBER_PASSWORD_UUID.to_string(),
        };
        // Assert that the application password is in DB
        assert!(
            db_application_password_meta_for_user(&mut db, &SECOND_USER_ID)
                .await
                .unwrap()
                .meta_value
                .contains(TEST_CREDENTIALS_SUBSCRIBER_PASSWORD_UUID)
        );
        // Delete the user's application passwords using the API and ensure it's successful
        let response = api_client()
            .application_passwords()
            .delete(&SECOND_USER_ID, &uuid)
            .await
            .assert_response();

        // Assert that the application password is deleted and no longer in DB
        assert!(response.deleted);
        assert_eq!(response.previous.uuid, uuid);
        assert!(
            !db_application_password_meta_for_user(&mut db, &SECOND_USER_ID)
                .await
                .unwrap()
                .meta_value
                .contains(TEST_CREDENTIALS_SUBSCRIBER_PASSWORD_UUID)
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn delete_all_application_passwords() {
    wp_db::run_and_restore(|mut db| async move {
        // Assert that the application password is in DB
        assert!(
            db_application_password_meta_for_user(&mut db, &SECOND_USER_ID)
                .await
                .unwrap()
                .meta_value
                .contains(TEST_CREDENTIALS_SUBSCRIBER_PASSWORD_UUID)
        );
        // Delete the user's application passwords using the API and ensure it's successful
        let response = api_client()
            .application_passwords()
            .delete_all(&SECOND_USER_ID)
            .await
            .assert_response();

        // Assert that the application password is deleted and no longer in DB
        assert!(response.deleted);
        assert_eq!(response.count, 1);
        assert!(
            !db_application_password_meta_for_user(&mut db, &SECOND_USER_ID)
                .await
                .unwrap()
                .meta_value
                .contains(TEST_CREDENTIALS_SUBSCRIBER_PASSWORD_UUID)
        );
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
