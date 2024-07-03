use integration_test_common::{request_builder_as_subscriber, request_builder_as_unauthenticated};
use rstest::*;
use serial_test::parallel;
use wp_api::application_passwords::{
    ApplicationPasswordCreateParams, ApplicationPasswordUpdateParams, ApplicationPasswordUuid,
};
use wp_api::WpRestErrorCode;

use crate::integration_test_common::{
    request_builder, AssertWpError, FIRST_USER_ID, SECOND_USER_ID,
    TEST_CREDENTIALS_ADMIN_PASSWORD_UUID,
};

pub mod integration_test_common;
pub mod reusable_test_cases;
pub mod wp_db;

#[rstest]
#[tokio::test]
#[parallel]
async fn list_application_passwords_err_cannot_list_application_passwords() {
    // Second user (subscriber) doesn't have access to the first users' application passwords
    request_builder_as_subscriber()
        .application_passwords()
        .list_with_edit_context(&FIRST_USER_ID)
        .await
        .assert_wp_error(WpRestErrorCode::CannotListApplicationPasswords);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_application_password_err_cannot_read_application_password() {
    // Second user (subscriber) doesn't have access to the first users' application passwords
    request_builder_as_subscriber()
        .application_passwords()
        .retrieve_with_edit_context(
            &FIRST_USER_ID,
            &ApplicationPasswordUuid {
                uuid: FIRST_USER_ID.to_string(),
            },
        )
        .await
        .assert_wp_error(WpRestErrorCode::CannotReadApplicationPassword);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn create_application_password_err_cannot_create_application_passwords() {
    // Second user (subscriber) can not create an application password for the first user
    request_builder_as_subscriber()
        .application_passwords()
        .create(
            &FIRST_USER_ID,
            &ApplicationPasswordCreateParams {
                app_id: None,
                name: "foo".to_string(),
            },
        )
        .await
        .assert_wp_error(WpRestErrorCode::CannotCreateApplicationPasswords);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn update_application_password_err_cannot_edit_application_password() {
    // Second user (subscriber) can not update an application password of the first user
    request_builder_as_subscriber()
        .application_passwords()
        .update(
            &FIRST_USER_ID,
            &ApplicationPasswordUuid {
                uuid: TEST_CREDENTIALS_ADMIN_PASSWORD_UUID.to_string(),
            },
            &ApplicationPasswordUpdateParams {
                app_id: None,
                name: "foo".to_string(),
            },
        )
        .await
        .assert_wp_error(WpRestErrorCode::CannotEditApplicationPassword);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn delete_application_password_err_cannot_delete_application_password() {
    // Second user (subscriber) can not delete an application password of the first user
    request_builder_as_subscriber()
        .application_passwords()
        .delete(
            &FIRST_USER_ID,
            &ApplicationPasswordUuid {
                uuid: TEST_CREDENTIALS_ADMIN_PASSWORD_UUID.to_string(),
            },
        )
        .await
        .assert_wp_error(WpRestErrorCode::CannotDeleteApplicationPassword);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn delete_application_passwords_err_cannot_delete_application_passwords() {
    // Second user (subscriber) can not delete all application passwords of the first user
    request_builder_as_subscriber()
        .application_passwords()
        .delete_all(&FIRST_USER_ID)
        .await
        .assert_wp_error(WpRestErrorCode::CannotDeleteApplicationPasswords);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_application_password_err_cannot_introspect_app_password_for_non_authenticated_user_401(
) {
    // Unauthenticated user can not retrieve the current application password for the second user
    request_builder_as_unauthenticated()
        .application_passwords()
        .retrieve_current_with_edit_context(&SECOND_USER_ID)
        .await
        .assert_wp_error(WpRestErrorCode::CannotIntrospectAppPasswordForNonAuthenticatedUser);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_application_password_err_cannot_introspect_app_password_for_another_user_403() {
    // First user can not retrieve the current application password for the second user
    request_builder()
        .application_passwords()
        .retrieve_current_with_edit_context(&SECOND_USER_ID)
        .await
        .assert_wp_error(WpRestErrorCode::CannotIntrospectAppPasswordForNonAuthenticatedUser);
}
