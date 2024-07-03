// TODO
#![allow(unused)]
use integration_test_common::request_builder_as_subscriber;
use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::application_passwords::{
    ApplicationPasswordUuid, SparseApplicationPassword, SparseApplicationPasswordField,
};
use wp_api::users::UserId;
use wp_api::{WpContext, WpRestErrorCode};

use crate::integration_test_common::{
    request_builder, AssertWpError, FIRST_USER_ID, SECOND_USER_ID,
    TEST_CREDENTIALS_ADMIN_PASSWORD_UUID, TEST_CREDENTIALS_SUBSCRIBER_PASSWORD_UUID,
};

pub mod integration_test_common;
pub mod reusable_test_cases;

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
