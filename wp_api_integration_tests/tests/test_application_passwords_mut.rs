use serial_test::serial;
use wp_api::{
    application_passwords::{
        ApplicationPasswordCreateParams, ApplicationPasswordUpdateParams, ApplicationPasswordUuid,
    },
    users::UserId,
};
use wp_api_integration_tests::Backend;
use wp_api_integration_tests::{
    api_client, AssertResponse, ServerRestore, FIRST_USER_ID, SECOND_USER_ID,
    TEST_CREDENTIALS_ADMIN_PASSWORD_UUID, TEST_CREDENTIALS_SUBSCRIBER_PASSWORD_UUID,
};
use wp_cli::WpCliUserMeta;

#[tokio::test]
#[serial]
async fn create_application_password() {
    let password_name = "IntegrationTest";
    // Assert that the application password name doesn't exist
    assert!(!application_password_meta_for_user(&SECOND_USER_ID)
        .await
        .unwrap()
        .meta_value
        .contains(password_name));

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

    // Assert that the application password is created
    let user_meta_after_update = application_password_meta_for_user(&SECOND_USER_ID).await;
    assert!(user_meta_after_update.is_some());
    let meta_value = user_meta_after_update.unwrap().meta_value;
    assert!(meta_value.contains(password_name));
    assert!(meta_value.contains(&created_application_password.uuid.uuid));

    ServerRestore::db().await;
}

#[tokio::test]
#[serial]
async fn update_application_password() {
    let password_name = "IntegrationTest";
    // Assert that the application password name doesn't exist
    assert!(!application_password_meta_for_user(&FIRST_USER_ID)
        .await
        .unwrap()
        .meta_value
        .contains(password_name));

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

    // Assert that the application password is created
    let user_meta_after_update = application_password_meta_for_user(&FIRST_USER_ID).await;
    assert!(user_meta_after_update.is_some());
    let meta_value = user_meta_after_update.unwrap().meta_value;
    assert!(meta_value.contains(password_name));
    assert!(meta_value.contains(&created_application_password.uuid.uuid));

    ServerRestore::db().await;
}

#[tokio::test]
#[serial]
async fn delete_single_application_password() {
    let uuid = ApplicationPasswordUuid {
        uuid: TEST_CREDENTIALS_SUBSCRIBER_PASSWORD_UUID.to_string(),
    };
    // Assert that the application password exists
    assert!(application_password_meta_for_user(&SECOND_USER_ID)
        .await
        .unwrap()
        .meta_value
        .contains(TEST_CREDENTIALS_SUBSCRIBER_PASSWORD_UUID));
    // Delete the user's application passwords using the API and ensure it's successful
    let response = api_client()
        .application_passwords()
        .delete(&SECOND_USER_ID, &uuid)
        .await
        .assert_response();

    // Assert that the application password is deleted
    assert!(response.deleted);
    assert_eq!(response.previous.uuid, uuid);
    assert!(!application_password_meta_for_user(&SECOND_USER_ID)
        .await
        .unwrap()
        .meta_value
        .contains(TEST_CREDENTIALS_SUBSCRIBER_PASSWORD_UUID));

    ServerRestore::db().await;
}

#[tokio::test]
#[serial]
async fn delete_all_application_passwords() {
    // Assert that the application password exists
    assert!(application_password_meta_for_user(&SECOND_USER_ID)
        .await
        .unwrap()
        .meta_value
        .contains(TEST_CREDENTIALS_SUBSCRIBER_PASSWORD_UUID));
    // Delete the user's application passwords using the API and ensure it's successful
    let response = api_client()
        .application_passwords()
        .delete_all(&SECOND_USER_ID)
        .await
        .assert_response();

    // Assert that the application password is deleted
    assert!(response.deleted);
    assert_eq!(response.count, 1);
    assert!(!application_password_meta_for_user(&SECOND_USER_ID)
        .await
        .unwrap()
        .meta_value
        .contains(TEST_CREDENTIALS_SUBSCRIBER_PASSWORD_UUID));

    ServerRestore::db().await;
}

async fn application_password_meta_for_user(user_id: &UserId) -> Option<WpCliUserMeta> {
    Backend::user_meta(user_id)
        .await
        .into_iter()
        .find(|m| m.meta_key == "_application_passwords")
}
