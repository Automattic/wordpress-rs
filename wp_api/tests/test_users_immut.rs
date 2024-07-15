use reusable_test_cases::list_users_cases;
use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::{
    generate,
    users::{
        SparseUserFieldWithEditContext, SparseUserWithEditContext, UserId, UserListParams,
        WpApiParamUsersHasPublishedPosts, WpApiParamUsersOrderBy, WpApiParamUsersWho,
    },
    WpApiParamOrder,
};

use crate::integration_test_common::{api_client, AssertResponse, FIRST_USER_ID, SECOND_USER_ID};

pub mod integration_test_common;
pub mod reusable_test_cases;

#[apply(filter_fields_cases_with_edit_context)]
#[tokio::test]
#[parallel]
async fn filter_users(#[case] fields: &[SparseUserFieldWithEditContext]) {
    api_client()
        .users()
        .filter_list_with_edit_context(&UserListParams::default(), fields)
        .await
        .assert_response()
        .iter()
        .for_each(|user| validate_sparse_user_fields(user, fields));
}

#[apply(filter_fields_cases_with_edit_context)]
#[tokio::test]
#[parallel]
async fn filter_retrieve_user(#[case] fields: &[SparseUserFieldWithEditContext]) {
    let user = api_client()
        .users()
        .filter_retrieve_with_edit_context(&FIRST_USER_ID, fields)
        .await
        .assert_response();
    validate_sparse_user_fields(&user, fields);
}

#[apply(filter_fields_cases_with_edit_context)]
#[tokio::test]
#[parallel]
async fn filter_retrieve_current_user(#[case] fields: &[SparseUserFieldWithEditContext]) {
    let user = api_client()
        .users()
        .filter_retrieve_me_with_edit_context(fields)
        .await
        .assert_response();
    validate_sparse_user_fields(&user, fields);
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

fn validate_sparse_user_fields(
    user: &SparseUserWithEditContext,
    fields: &[SparseUserFieldWithEditContext],
) {
    let field_included = |field| {
        // If "fields" is empty the server will return all fields
        fields.is_empty() || fields.contains(&field)
    };
    assert_eq!(
        user.id.is_some(),
        field_included(SparseUserFieldWithEditContext::Id)
    );
    assert_eq!(
        user.username.is_some(),
        field_included(SparseUserFieldWithEditContext::Username)
    );
    assert_eq!(
        user.name.is_some(),
        field_included(SparseUserFieldWithEditContext::Name)
    );
    assert_eq!(
        user.last_name.is_some(),
        field_included(SparseUserFieldWithEditContext::LastName)
    );
    assert_eq!(
        user.email.is_some(),
        field_included(SparseUserFieldWithEditContext::Email)
    );
    assert_eq!(
        user.url.is_some(),
        field_included(SparseUserFieldWithEditContext::Url)
    );
    assert_eq!(
        user.description.is_some(),
        field_included(SparseUserFieldWithEditContext::Description)
    );
    assert_eq!(
        user.link.is_some(),
        field_included(SparseUserFieldWithEditContext::Link)
    );
    assert_eq!(
        user.locale.is_some(),
        field_included(SparseUserFieldWithEditContext::Locale)
    );
    assert_eq!(
        user.nickname.is_some(),
        field_included(SparseUserFieldWithEditContext::Nickname)
    );
    assert_eq!(
        user.slug.is_some(),
        field_included(SparseUserFieldWithEditContext::Slug)
    );
    assert_eq!(
        user.registered_date.is_some(),
        field_included(SparseUserFieldWithEditContext::RegisteredDate)
    );
    assert_eq!(
        user.roles.is_some(),
        field_included(SparseUserFieldWithEditContext::Roles)
    );
    assert_eq!(
        user.capabilities.is_some(),
        field_included(SparseUserFieldWithEditContext::Capabilities)
    );
    assert_eq!(
        user.extra_capabilities.is_some(),
        field_included(SparseUserFieldWithEditContext::ExtraCapabilities)
    );
    assert_eq!(
        user.avatar_urls.is_some(),
        field_included(SparseUserFieldWithEditContext::AvatarUrls)
    );
}

#[template]
#[rstest]
#[case(None)]
#[case(Some(WpApiParamUsersHasPublishedPosts::True))]
#[case(Some(WpApiParamUsersHasPublishedPosts::False))]
#[case(Some(WpApiParamUsersHasPublishedPosts::PostTypes(vec!["post".to_string()])))]
#[case(Some(WpApiParamUsersHasPublishedPosts::PostTypes(vec!["post".to_string(), "page".to_string()])))]
fn list_users_has_published_posts_cases() {}

#[template]
#[rstest]
#[case(&[])]
#[case(&[SparseUserFieldWithEditContext::Id])]
#[case(&[SparseUserFieldWithEditContext::Username])]
#[case(&[SparseUserFieldWithEditContext::Name])]
#[case(&[SparseUserFieldWithEditContext::LastName])]
#[case(&[SparseUserFieldWithEditContext::Email])]
#[case(&[SparseUserFieldWithEditContext::Url])]
#[case(&[SparseUserFieldWithEditContext::Description])]
#[case(&[SparseUserFieldWithEditContext::Link])]
#[case(&[SparseUserFieldWithEditContext::Locale])]
#[case(&[SparseUserFieldWithEditContext::Nickname])]
#[case(&[SparseUserFieldWithEditContext::Slug])]
#[case(&[SparseUserFieldWithEditContext::RegisteredDate])]
#[case(&[SparseUserFieldWithEditContext::Roles])]
#[case(&[SparseUserFieldWithEditContext::Capabilities])]
#[case(&[SparseUserFieldWithEditContext::ExtraCapabilities])]
#[case(&[SparseUserFieldWithEditContext::AvatarUrls])]
#[case(&[SparseUserFieldWithEditContext::Id, SparseUserFieldWithEditContext::Name])]
#[case(&[SparseUserFieldWithEditContext::Email, SparseUserFieldWithEditContext::Nickname])]
fn filter_fields_cases_with_edit_context(#[case] fields: &[SparseUserFieldWithEditContext]) {}
