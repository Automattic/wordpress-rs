use rstest::*;
use rstest_reuse::{self, apply, template};
use wp_api::application_passwords::{SparseApplicationPassword, SparseApplicationPasswordField};
use wp_api::users::UserId;
use wp_api::WpContext;

use crate::integration_test_common::{
    request_builder, AssertResponse, FIRST_USER_ID, SECOND_USER_ID,
};

pub mod integration_test_common;
pub mod reusable_test_cases;

#[apply(filter_fields_cases)]
#[tokio::test]
async fn filter_application_passwords(
    #[values(FIRST_USER_ID, SECOND_USER_ID)] user_id: UserId,
    #[case] fields: &[SparseApplicationPasswordField],
) {
    request_builder()
        .application_passwords()
        .filter_list(&user_id, WpContext::Edit, fields)
        .await
        .assert_response()
        .iter()
        .for_each(|p| validate_sparse_application_password_fields(p, fields));
}

#[rstest]
#[tokio::test]
async fn list_application_passwords_with_edit_context(
    #[values(FIRST_USER_ID, SECOND_USER_ID)] user_id: UserId,
) {
    request_builder()
        .application_passwords()
        .list_with_edit_context(&user_id)
        .await
        .assert_response();
}

#[rstest]
#[tokio::test]
async fn list_application_passwords_with_embed_context(
    #[values(FIRST_USER_ID, SECOND_USER_ID)] user_id: UserId,
) {
    request_builder()
        .application_passwords()
        .list_with_embed_context(&user_id)
        .await
        .assert_response();
}

#[rstest]
#[tokio::test]
async fn list_application_passwords_with_view_context(
    #[values(FIRST_USER_ID, SECOND_USER_ID)] user_id: UserId,
) {
    request_builder()
        .application_passwords()
        .list_with_view_context(&user_id)
        .await
        .assert_response();
}

// TODO: This might not be a good test case to keep, but it's helpful during initial implementation
// to ensure that the ip address is properly parsed
#[tokio::test]
async fn list_application_passwords_ensure_last_ip() {
    let list = request_builder()
        .application_passwords()
        .list_with_edit_context(&FIRST_USER_ID)
        .await
        .assert_response();
    assert!(list.first().unwrap().last_ip.is_some());
}

fn validate_sparse_application_password_fields(
    app_password: &SparseApplicationPassword,
    fields: &[SparseApplicationPasswordField],
) {
    let field_included = |field| {
        // If "fields" is empty the server will return all fields
        fields.is_empty() || fields.contains(&field)
    };
    assert_eq!(
        app_password.uuid.is_some(),
        field_included(SparseApplicationPasswordField::Uuid)
    );
    assert_eq!(
        app_password.app_id.is_some(),
        field_included(SparseApplicationPasswordField::AppId)
    );
    assert_eq!(
        app_password.name.is_some(),
        field_included(SparseApplicationPasswordField::Name)
    );
    assert_eq!(
        app_password.created.is_some(),
        field_included(SparseApplicationPasswordField::Created)
    );
    // Do not test existence of `last_used`, `last_ip` or `password` as there is
    // no guarantee that they'll be included even if it's in the requested field list
    if !field_included(SparseApplicationPasswordField::LastUsed) {
        assert!(app_password.last_used.is_none());
    }
    if !field_included(SparseApplicationPasswordField::LastIp) {
        assert!(app_password.last_ip.is_none());
    }
    if !field_included(SparseApplicationPasswordField::Password) {
        assert!(app_password.password.is_none());
    }
}

#[template]
#[rstest]
#[case(&[])]
#[case(&[SparseApplicationPasswordField::Uuid])]
#[case(&[SparseApplicationPasswordField::AppId])]
#[case(&[SparseApplicationPasswordField::Name])]
#[case(&[SparseApplicationPasswordField::Created])]
#[case(&[SparseApplicationPasswordField::LastUsed])]
#[case(&[SparseApplicationPasswordField::LastIp])]
#[case(&[SparseApplicationPasswordField::Uuid, SparseApplicationPasswordField::Name])]
fn filter_fields_cases(#[case] fields: &[SparseApplicationPasswordField]) {}
