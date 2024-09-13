use rstest::*;
use serial_test::parallel;
use wp_api::application_passwords::{
    ApplicationPasswordCreateParams, ApplicationPasswordUpdateParams, ApplicationPasswordUuid,
};
use wp_api::WpErrorCode;

use wp_api_integration_tests::{
    api_client, api_client_as_subscriber, api_client_as_unauthenticated, AssertWpError,
    TestCredentials, FIRST_USER_ID, SECOND_USER_ID,
};

pub mod reusable_test_cases;

#[rstest]
#[tokio::test]
#[parallel]
async fn list_application_passwords_err_cannot_list_application_passwords() {
    // Second user (subscriber) doesn't have access to the first users' application passwords
    api_client_as_subscriber()
        .application_passwords()
        .list_with_edit_context(&FIRST_USER_ID)
        .await
        .assert_wp_error(WpErrorCode::CannotListApplicationPasswords);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_application_password_err_cannot_read_application_password() {
    // Second user (subscriber) doesn't have access to the first users' application passwords
    api_client_as_subscriber()
        .application_passwords()
        .retrieve_with_edit_context(
            &FIRST_USER_ID,
            &ApplicationPasswordUuid {
                uuid: FIRST_USER_ID.to_string(),
            },
        )
        .await
        .assert_wp_error(WpErrorCode::CannotReadApplicationPassword);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn create_application_password_err_cannot_create_application_passwords() {
    // Second user (subscriber) can not create an application password for the first user
    api_client_as_subscriber()
        .application_passwords()
        .create(
            &FIRST_USER_ID,
            &ApplicationPasswordCreateParams {
                app_id: None,
                name: "foo".to_string(),
            },
        )
        .await
        .assert_wp_error(WpErrorCode::CannotCreateApplicationPasswords);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn update_application_password_err_cannot_edit_application_password() {
    // Second user (subscriber) can not update an application password of the first user
    api_client_as_subscriber()
        .application_passwords()
        .update(
            &FIRST_USER_ID,
            &ApplicationPasswordUuid {
                uuid: TestCredentials::instance().admin_password_uuid,
            },
            &ApplicationPasswordUpdateParams {
                app_id: None,
                name: "foo".to_string(),
            },
        )
        .await
        .assert_wp_error(WpErrorCode::CannotEditApplicationPassword);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn delete_application_password_err_cannot_delete_application_password() {
    // Second user (subscriber) can not delete an application password of the first user
    api_client_as_subscriber()
        .application_passwords()
        .delete(
            &FIRST_USER_ID,
            &ApplicationPasswordUuid {
                uuid: TestCredentials::instance().admin_password_uuid,
            },
        )
        .await
        .assert_wp_error(WpErrorCode::CannotDeleteApplicationPassword);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn delete_application_passwords_err_cannot_delete_application_passwords() {
    // Second user (subscriber) can not delete all application passwords of the first user
    api_client_as_subscriber()
        .application_passwords()
        .delete_all(&FIRST_USER_ID)
        .await
        .assert_wp_error(WpErrorCode::CannotDeleteApplicationPasswords);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_application_password_err_cannot_introspect_app_password_for_non_authenticated_user_401(
) {
    // Unauthenticated user can not retrieve the current application password for the second user
    api_client_as_unauthenticated()
        .application_passwords()
        .retrieve_current_with_edit_context(&SECOND_USER_ID)
        .await
        .assert_wp_error(WpErrorCode::CannotIntrospectAppPasswordForNonAuthenticatedUser);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_application_password_err_cannot_introspect_app_password_for_another_user_403() {
    // First user can not retrieve the current application password for the second user
    api_client()
        .application_passwords()
        .retrieve_current_with_edit_context(&SECOND_USER_ID)
        .await
        .assert_wp_error(WpErrorCode::CannotIntrospectAppPasswordForNonAuthenticatedUser);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_application_password_err_application_password_not_found() {
    api_client()
        .application_passwords()
        .retrieve_with_edit_context(
            &FIRST_USER_ID,
            &ApplicationPasswordUuid {
                uuid: "foo".to_string(),
            },
        )
        .await
        .assert_wp_error(WpErrorCode::ApplicationPasswordNotFound);
}
