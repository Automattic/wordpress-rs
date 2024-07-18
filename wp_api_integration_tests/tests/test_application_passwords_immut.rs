use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::application_passwords::{
    ApplicationPasswordUuid, SparseApplicationPasswordFieldWithEditContext,
    SparseApplicationPasswordWithEditContext,
};
use wp_api::generate_sparse_application_password_field_with_edit_context_test_cases;
use wp_api::users::UserId;
use wp_api_integration_tests::{
    api_client, api_client_as_subscriber, AssertResponse, FIRST_USER_ID, SECOND_USER_ID,
    TEST_CREDENTIALS_ADMIN_PASSWORD_UUID, TEST_CREDENTIALS_SUBSCRIBER_PASSWORD_UUID,
};

pub mod reusable_test_cases;

generate_sparse_application_password_field_with_edit_context_test_cases!();

#[apply(sparse_application_password_field_with_edit_context_test_cases)]
#[case(&[SparseApplicationPasswordFieldWithEditContext::Uuid, SparseApplicationPasswordFieldWithEditContext::Name])]
#[tokio::test]
#[parallel]
async fn filter_list_application_passwords_with_edit_context(
    #[values(FIRST_USER_ID, SECOND_USER_ID)] user_id: UserId,
    #[case] fields: &[SparseApplicationPasswordFieldWithEditContext],
) {
    if should_skip_filter_with_edit_context_test(fields) {
        return;
    }
    api_client()
        .application_passwords()
        .filter_list_with_edit_context(&user_id, fields)
        .await
        .assert_response()
        .iter()
        .for_each(|p| p.assert_that_instance_fields_nullability_match_provided_fields(fields));
}

#[apply(sparse_application_password_field_with_edit_context_test_cases)]
#[case(&[SparseApplicationPasswordFieldWithEditContext::Uuid, SparseApplicationPasswordFieldWithEditContext::Name])]
#[tokio::test]
#[parallel]
async fn filter_retrieve_application_password_with_edit_context(
    #[case] fields: &[SparseApplicationPasswordFieldWithEditContext],
) {
    if should_skip_filter_with_edit_context_test(fields) {
        return;
    }
    let p: SparseApplicationPasswordWithEditContext = api_client()
        .application_passwords()
        .filter_retrieve_with_edit_context(
            &FIRST_USER_ID,
            &ApplicationPasswordUuid {
                uuid: TEST_CREDENTIALS_ADMIN_PASSWORD_UUID.to_string(),
            },
            fields,
        )
        .await
        .assert_response();
    p.assert_that_instance_fields_nullability_match_provided_fields(fields);
}

#[apply(sparse_application_password_field_with_edit_context_test_cases)]
#[case(&[SparseApplicationPasswordFieldWithEditContext::Uuid, SparseApplicationPasswordFieldWithEditContext::Name])]
#[tokio::test]
#[parallel]
async fn filter_retrieve_current_application_password_with_edit_context(
    #[case] fields: &[SparseApplicationPasswordFieldWithEditContext],
) {
    if should_skip_filter_with_edit_context_test(fields) {
        return;
    }
    let p = api_client()
        .application_passwords()
        .filter_retrieve_current_with_edit_context(&FIRST_USER_ID, fields)
        .await
        .assert_response();
    p.assert_that_instance_fields_nullability_match_provided_fields(fields);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn list_application_passwords_with_edit_context(
    #[values(FIRST_USER_ID, SECOND_USER_ID)] user_id: UserId,
) {
    api_client()
        .application_passwords()
        .list_with_edit_context(&user_id)
        .await
        .assert_response();
}

#[rstest]
#[tokio::test]
#[parallel]
async fn list_application_passwords_with_embed_context(
    #[values(FIRST_USER_ID, SECOND_USER_ID)] user_id: UserId,
) {
    api_client()
        .application_passwords()
        .list_with_embed_context(&user_id)
        .await
        .assert_response();
}

#[rstest]
#[tokio::test]
#[parallel]
async fn list_application_passwords_with_view_context(
    #[values(FIRST_USER_ID, SECOND_USER_ID)] user_id: UserId,
) {
    api_client()
        .application_passwords()
        .list_with_view_context(&user_id)
        .await
        .assert_response();
}

// TODO: This might not be a good test case to keep, but it's helpful during initial implementation
// to ensure that the ip address is properly parsed
#[tokio::test]
#[parallel]
async fn list_application_passwords_ensure_last_ip() {
    let list = api_client()
        .application_passwords()
        .list_with_edit_context(&FIRST_USER_ID)
        .await
        .assert_response();
    assert!(list.first().unwrap().last_ip.is_some());
}

#[tokio::test]
#[parallel]
async fn retrieve_current_application_passwords_with_edit_context() {
    let a = api_client()
        .application_passwords()
        .retrieve_current_with_edit_context(&FIRST_USER_ID)
        .await
        .assert_response();
    assert_eq!(
        a.uuid,
        ApplicationPasswordUuid {
            uuid: TEST_CREDENTIALS_ADMIN_PASSWORD_UUID.to_string()
        }
    );
}

#[tokio::test]
#[parallel]
async fn retrieve_current_application_passwords_with_embed_context() {
    let a = api_client_as_subscriber()
        .application_passwords()
        .retrieve_current_with_embed_context(&SECOND_USER_ID)
        .await
        .assert_response();
    assert_eq!(
        a.uuid,
        ApplicationPasswordUuid {
            uuid: TEST_CREDENTIALS_SUBSCRIBER_PASSWORD_UUID.to_string()
        }
    );
}

#[tokio::test]
#[parallel]
async fn retrieve_current_application_passwords_with_view_context() {
    let a = api_client()
        .application_passwords()
        .retrieve_current_with_view_context(&FIRST_USER_ID)
        .await
        .assert_response();
    assert_eq!(
        a.uuid,
        ApplicationPasswordUuid {
            uuid: TEST_CREDENTIALS_ADMIN_PASSWORD_UUID.to_string()
        }
    );
}

#[tokio::test]
#[parallel]
async fn retrieve_application_passwords_with_edit_context() {
    let uuid = ApplicationPasswordUuid {
        uuid: TEST_CREDENTIALS_ADMIN_PASSWORD_UUID.to_string(),
    };
    let a = api_client()
        .application_passwords()
        .retrieve_with_edit_context(&FIRST_USER_ID, &uuid)
        .await
        .assert_response();
    assert_eq!(a.uuid, uuid);
}

#[tokio::test]
#[parallel]
async fn retrieve_application_passwords_with_embed_context() {
    let uuid = ApplicationPasswordUuid {
        uuid: TEST_CREDENTIALS_ADMIN_PASSWORD_UUID.to_string(),
    };
    let a = api_client()
        .application_passwords()
        .retrieve_with_embed_context(&FIRST_USER_ID, &uuid)
        .await
        .assert_response();
    assert_eq!(a.uuid, uuid);
}

#[tokio::test]
#[parallel]
async fn retrieve_application_passwords_with_view_context() {
    let uuid = ApplicationPasswordUuid {
        uuid: TEST_CREDENTIALS_SUBSCRIBER_PASSWORD_UUID.to_string(),
    };
    let a = api_client()
        .application_passwords()
        .retrieve_with_view_context(&SECOND_USER_ID, &uuid)
        .await
        .assert_response();
    assert_eq!(a.uuid, uuid);
}

fn should_skip_filter_with_edit_context_test(
    fields: &[SparseApplicationPasswordFieldWithEditContext],
) -> bool {
    if fields.contains(&SparseApplicationPasswordFieldWithEditContext::Password) {
        println!(
            "Requesting password field returns invalid JSON as this field is only available after creating a new application token. Skipping this test..."
        );
        true
    } else {
        false
    }
}
