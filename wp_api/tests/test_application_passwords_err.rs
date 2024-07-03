// TODO
#![allow(unused)]
use integration_test_common::{request_builder_as_subscriber, run_wp_cli_command};
use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::{parallel, serial};
use wp_api::application_passwords::{
    ApplicationPasswordCreateParams, ApplicationPasswordUpdateParams, ApplicationPasswordUuid,
    SparseApplicationPassword, SparseApplicationPasswordField,
};
use wp_api::users::UserId;
use wp_api::{WpContext, WpRestErrorCode};

use crate::integration_test_common::{
    request_builder, AssertWpError, FIRST_USER_ID, SECOND_USER_ID,
    TEST_CREDENTIALS_ADMIN_PASSWORD_UUID, TEST_CREDENTIALS_SUBSCRIBER_PASSWORD_UUID,
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
