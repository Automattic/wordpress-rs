use reusable_test_cases::list_users_cases;
use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::{
    generate, generate_sparse_user_field_with_edit_context_test_cases,
    users::{
        SparseUserFieldWithEditContext, UserId, UserListParams, WpApiParamUsersHasPublishedPosts,
        WpApiParamUsersOrderBy, WpApiParamUsersWho,
    },
    WpApiParamOrder,
};

use crate::integration_test_common::{api_client, AssertResponse, FIRST_USER_ID, SECOND_USER_ID};

pub mod integration_test_common;
pub mod reusable_test_cases;

generate_sparse_user_field_with_edit_context_test_cases!();

#[apply(sparse_user_field_with_edit_context_test_cases)]
#[case(&[SparseUserFieldWithEditContext::Id, SparseUserFieldWithEditContext::Name])]
#[case(&[SparseUserFieldWithEditContext::Email, SparseUserFieldWithEditContext::Nickname])]
#[tokio::test]
#[parallel]
async fn filter_users(#[case] fields: &[SparseUserFieldWithEditContext]) {
    api_client()
        .users()
        .filter_list_with_edit_context(&UserListParams::default(), fields)
        .await
        .assert_response()
        .iter()
        .for_each(|user| user.assert_that_only_provided_fields_are_some(fields));
}

#[apply(sparse_user_field_with_edit_context_test_cases)]
#[case(&[SparseUserFieldWithEditContext::Id, SparseUserFieldWithEditContext::Name])]
#[case(&[SparseUserFieldWithEditContext::Email, SparseUserFieldWithEditContext::Nickname])]
#[tokio::test]
#[parallel]
async fn filter_retrieve_user(#[case] fields: &[SparseUserFieldWithEditContext]) {
    let user = api_client()
        .users()
        .filter_retrieve_with_edit_context(&FIRST_USER_ID, fields)
        .await
        .assert_response();
    user.assert_that_only_provided_fields_are_some(fields);
}

#[apply(sparse_user_field_with_edit_context_test_cases)]
#[case(&[SparseUserFieldWithEditContext::Id, SparseUserFieldWithEditContext::Name])]
#[case(&[SparseUserFieldWithEditContext::Email, SparseUserFieldWithEditContext::Nickname])]
#[tokio::test]
#[parallel]
async fn filter_retrieve_current_user(#[case] fields: &[SparseUserFieldWithEditContext]) {
    let user = api_client()
        .users()
        .filter_retrieve_me_with_edit_context(fields)
        .await
        .assert_response();
    user.assert_that_only_provided_fields_are_some(fields);
}

#[apply(list_users_cases)]
#[tokio::test]
#[parallel]
async fn list_users_with_edit_context(#[case] params: UserListParams) {
    api_client()
        .users()
        .list_with_edit_context(&params)
        .await
        .assert_response();
}

#[apply(list_users_cases)]
#[tokio::test]
#[parallel]
async fn list_users_with_embed_context(#[case] params: UserListParams) {
    api_client()
        .users()
        .list_with_embed_context(&params)
        .await
        .assert_response();
}

#[apply(list_users_cases)]
#[tokio::test]
#[parallel]
async fn list_users_with_view_context(#[case] params: UserListParams) {
    api_client()
        .users()
        .list_with_view_context(&params)
        .await
        .assert_response();
}

#[apply(list_users_has_published_posts_cases)]
#[trace]
#[tokio::test]
#[parallel]
async fn list_users_with_edit_context_has_published_posts(
    #[case] has_published_posts: Option<WpApiParamUsersHasPublishedPosts>,
) {
    api_client()
        .users()
        .list_with_edit_context(&UserListParams {
            has_published_posts,
            ..Default::default()
        })
        .await
        .assert_response();
}

#[apply(list_users_has_published_posts_cases)]
#[trace]
#[tokio::test]
#[parallel]
async fn list_users_with_embed_context_has_published_posts(
    #[case] has_published_posts: Option<WpApiParamUsersHasPublishedPosts>,
) {
    api_client()
        .users()
        .list_with_embed_context(&UserListParams {
            has_published_posts,
            ..Default::default()
        })
        .await
        .assert_response();
}

#[apply(list_users_has_published_posts_cases)]
#[trace]
#[tokio::test]
#[parallel]
async fn list_users_with_view_context_has_published_posts(
    #[case] has_published_posts: Option<WpApiParamUsersHasPublishedPosts>,
) {
    api_client()
        .users()
        .list_with_view_context(&UserListParams {
            has_published_posts,
            ..Default::default()
        })
        .await
        .assert_response();
}

#[rstest]
#[trace]
#[tokio::test]
#[parallel]
async fn retrieve_user_with_edit_context(#[values(FIRST_USER_ID, SECOND_USER_ID)] user_id: UserId) {
    let user = api_client()
        .users()
        .retrieve_with_edit_context(&user_id)
        .await
        .assert_response();
    assert_eq!(user_id, user.id);
}

#[rstest]
#[trace]
#[tokio::test]
#[parallel]
async fn retrieve_user_with_embed_context(
    #[values(FIRST_USER_ID, SECOND_USER_ID)] user_id: UserId,
) {
    let user = api_client()
        .users()
        .retrieve_with_embed_context(&user_id)
        .await
        .assert_response();
    assert_eq!(user_id, user.id);
}

#[rstest]
#[trace]
#[tokio::test]
#[parallel]
async fn retrieve_user_with_view_context(#[values(FIRST_USER_ID, SECOND_USER_ID)] user_id: UserId) {
    let user = api_client()
        .users()
        .retrieve_with_view_context(&user_id)
        .await
        .assert_response();
    assert_eq!(user_id, user.id);
}

#[tokio::test]
#[parallel]
async fn retrieve_me_with_edit_context() {
    let user = api_client()
        .users()
        .retrieve_me_with_edit_context()
        .await
        .assert_response();
    // FIRST_USER_ID is the current user's id
    assert_eq!(FIRST_USER_ID, user.id);
}

#[tokio::test]
#[parallel]
async fn retrieve_me_with_embed_context() {
    let user = api_client()
        .users()
        .retrieve_me_with_embed_context()
        .await
        .assert_response();
    // FIRST_USER_ID is the current user's id
    assert_eq!(FIRST_USER_ID, user.id);
}

#[tokio::test]
#[parallel]
async fn retrieve_me_with_view_context() {
    let user = api_client()
        .users()
        .retrieve_me_with_view_context()
        .await
        .assert_response();
    // FIRST_USER_ID is the current user's id
    assert_eq!(FIRST_USER_ID, user.id);
}

#[template]
#[rstest]
#[case(None)]
#[case(Some(WpApiParamUsersHasPublishedPosts::True))]
#[case(Some(WpApiParamUsersHasPublishedPosts::False))]
#[case(Some(WpApiParamUsersHasPublishedPosts::PostTypes(vec!["post".to_string()])))]
#[case(Some(WpApiParamUsersHasPublishedPosts::PostTypes(vec!["post".to_string(), "page".to_string()])))]
fn list_users_has_published_posts_cases() {}
