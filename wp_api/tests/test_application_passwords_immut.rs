use integration_test_common::api_client_as_subscriber;
use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::application_passwords::{
    ApplicationPasswordUuid, SparseApplicationPasswordFieldWithEditContext,
    SparseApplicationPasswordWithEditContext,
};
use wp_api::users::UserId;

use crate::integration_test_common::{
    api_client, AssertResponse, FIRST_USER_ID, SECOND_USER_ID,
    TEST_CREDENTIALS_ADMIN_PASSWORD_UUID, TEST_CREDENTIALS_SUBSCRIBER_PASSWORD_UUID,
};

pub mod integration_test_common;
pub mod reusable_test_cases;

#[apply(filter_fields_cases_with_edit_context)]
#[tokio::test]
#[parallel]
async fn filter_list_application_passwords(
    #[values(FIRST_USER_ID, SECOND_USER_ID)] user_id: UserId,
    #[case] fields: &[SparseApplicationPasswordFieldWithEditContext],
) {
    api_client()
        .application_passwords()
        .filter_list_with_edit_context(&user_id, fields)
        .await
        .assert_response()
        .iter()
        .for_each(|p| validate_sparse_application_password_fields(p, fields));
}

#[apply(filter_fields_cases_with_edit_context)]
#[tokio::test]
#[parallel]
async fn filter_retrieve_application_password(
    #[case] fields: &[SparseApplicationPasswordFieldWithEditContext],
) {
    let p = api_client()
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
    validate_sparse_application_password_fields(&p, fields);
}

#[apply(filter_fields_cases_with_edit_context)]
#[tokio::test]
#[parallel]
async fn filter_retrieve_current_application_password(
    #[case] fields: &[SparseApplicationPasswordFieldWithEditContext],
) {
    let p = api_client()
        .application_passwords()
        .filter_retrieve_current_with_edit_context(&FIRST_USER_ID, fields)
        .await
        .assert_response();
    validate_sparse_application_password_fields(&p, fields);
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

fn validate_sparse_application_password_fields(
    app_password: &SparseApplicationPasswordWithEditContext,
    fields: &[SparseApplicationPasswordFieldWithEditContext],
) {
    let field_included = |field| {
        // If "fields" is empty the server will return all fields
        fields.is_empty() || fields.contains(&field)
    };
    assert_eq!(
        app_password.uuid.is_some(),
        field_included(SparseApplicationPasswordFieldWithEditContext::Uuid)
    );
    assert_eq!(
        app_password.app_id.is_some(),
        field_included(SparseApplicationPasswordFieldWithEditContext::AppId)
    );
    assert_eq!(
        app_password.name.is_some(),
        field_included(SparseApplicationPasswordFieldWithEditContext::Name)
    );
    assert_eq!(
        app_password.created.is_some(),
        field_included(SparseApplicationPasswordFieldWithEditContext::Created)
    );
    // Do not test existence of `last_used`, `last_ip` or `password` as there is
    // no guarantee that they'll be included even if it's in the requested field list
    if !field_included(SparseApplicationPasswordFieldWithEditContext::LastUsed) {
        assert!(app_password.last_used.is_none());
    }
    if !field_included(SparseApplicationPasswordFieldWithEditContext::LastIp) {
        assert!(app_password.last_ip.is_none());
    }
    if !field_included(SparseApplicationPasswordFieldWithEditContext::Password) {
        assert!(app_password.password.is_none());
    }
}

#[template]
#[rstest]
#[case(&[])]
#[case(&[SparseApplicationPasswordFieldWithEditContext::Uuid])]
#[case(&[SparseApplicationPasswordFieldWithEditContext::AppId])]
#[case(&[SparseApplicationPasswordFieldWithEditContext::Name])]
#[case(&[SparseApplicationPasswordFieldWithEditContext::Created])]
#[case(&[SparseApplicationPasswordFieldWithEditContext::LastUsed])]
#[case(&[SparseApplicationPasswordFieldWithEditContext::LastIp])]
#[case(&[SparseApplicationPasswordFieldWithEditContext::Uuid, SparseApplicationPasswordFieldWithEditContext::Name])]
fn filter_fields_cases_with_edit_context(
    #[case] fields: &[SparseApplicationPasswordFieldWithEditContext],
) {
}
