use reusable_test_cases::list_users_cases;
use wp_api::users::{
    SparseUserFieldWithEditContext, SparseUserFieldWithEmbedContext,
    SparseUserFieldWithViewContext, UserId, UserListParams, WpApiParamUsersHasPublishedPosts,
    WpApiParamUsersOrderBy, WpApiParamUsersWho,
};
use wp_api_integration_tests::prelude::*;

pub mod reusable_test_cases;

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
        .assert_response()
        .data;
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
        .assert_response()
        .data;
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
        .assert_response()
        .data;
    assert_eq!(user_id, user.id);
}

// Regression test for issue #1313: legacy WordPress sites can store the role assignment
// in `wp_capabilities` as the string `"1"` instead of boolean `true`. The fixture user
// `legacy_admin` has its `wp_capabilities` poisoned to `{"administrator": "1"}` so the
// REST response carries the legacy string shape on `extra_capabilities` (raw user meta)
// and on `capabilities` (allcaps merged with the user-level overlay).
#[tokio::test]
#[parallel]
async fn retrieve_legacy_admin_with_edit_context_parses() {
    let user_id = UserId(TestCredentials::instance().legacy_admin_user_id);
    let user = api_client()
        .users()
        .retrieve_with_edit_context(&user_id)
        .await
        .assert_response()
        .data;
    assert_eq!(user_id, user.id);
}

// Regression test for PR #1263: plugins like WPBakery call `WP_User::add_cap($cap, $grant)`
// with non-bool grants, leaving entries such as `"vc_access_rules_post_types": "custom"` in
// `wp_capabilities`. The fixture user `wpbakery_admin` carries both a boolean role entry
// and a plugin-style string entry, exercising the deserializer on both response fields.
#[tokio::test]
#[parallel]
async fn retrieve_wpbakery_admin_with_edit_context_parses() {
    let user_id = UserId(TestCredentials::instance().wpbakery_admin_user_id);
    let user = api_client()
        .users()
        .retrieve_with_edit_context(&user_id)
        .await
        .assert_response()
        .data;
    assert_eq!(user_id, user.id);
}

#[tokio::test]
#[parallel]
async fn retrieve_me_with_edit_context() {
    let user = api_client()
        .users()
        .retrieve_me_with_edit_context()
        .await
        .assert_response()
        .data;
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
        .assert_response()
        .data;
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
        .assert_response()
        .data;
    // FIRST_USER_ID is the current user's id
    assert_eq!(FIRST_USER_ID, user.id);
}

#[tokio::test]
#[rstest]
#[parallel]
#[case(UserListParams { per_page: Some(1), ..Default::default() })]
#[case(UserListParams { per_page: Some(1), order: Some(WpApiParamOrder::Desc), ..Default::default() })]
#[case(UserListParams { per_page: Some(1), orderby: Some(WpApiParamUsersOrderBy::Email), ..Default::default() })]
async fn paginate_list_users_with_edit_context(#[case] params: UserListParams) {
    let first_page_response = api_client()
        .users()
        .list_with_edit_context(&params)
        .await
        .assert_response();
    assert!(!first_page_response.data.is_empty());
    let next_page_params = first_page_response.next_page_params.unwrap();
    let next_page_response = api_client()
        .users()
        .list_with_edit_context(&next_page_params)
        .await
        .assert_response();
    assert!(!next_page_response.data.is_empty());
    let prev_page_params = next_page_response.prev_page_params.unwrap();
    let prev_page_response = api_client()
        .users()
        .list_with_edit_context(&prev_page_params)
        .await
        .assert_response();
    assert!(!prev_page_response.data.is_empty());
}

#[template]
#[rstest]
#[case(None)]
#[case(Some(WpApiParamUsersHasPublishedPosts::True))]
#[case(Some(WpApiParamUsersHasPublishedPosts::False))]
#[case(Some(WpApiParamUsersHasPublishedPosts::PostTypes(vec!["post".to_string()])))]
#[case(Some(WpApiParamUsersHasPublishedPosts::PostTypes(vec!["post".to_string(), "page".to_string()])))]
fn list_users_has_published_posts_cases() {}

mod filter {
    use super::*;

    wp_api::generate_sparse_user_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_user_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_user_field_with_view_context_test_cases!();

    #[apply(sparse_user_field_with_edit_context_test_cases)]
    #[case(&[SparseUserFieldWithEditContext::Id, SparseUserFieldWithEditContext::Name])]
    #[case(&[SparseUserFieldWithEditContext::Email, SparseUserFieldWithEditContext::Nickname])]
    #[tokio::test]
    #[parallel]
    async fn filter_users_with_edit_context(#[case] fields: &[SparseUserFieldWithEditContext]) {
        api_client()
            .users()
            .filter_list_with_edit_context(&UserListParams::default(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|user| {
                user.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_user_field_with_edit_context_test_cases)]
    #[case(&[SparseUserFieldWithEditContext::Id, SparseUserFieldWithEditContext::Name])]
    #[case(&[SparseUserFieldWithEditContext::Email, SparseUserFieldWithEditContext::Nickname])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_user_with_edit_context(
        #[case] fields: &[SparseUserFieldWithEditContext],
    ) {
        let user = api_client()
            .users()
            .filter_retrieve_with_edit_context(&FIRST_USER_ID, fields)
            .await
            .assert_response()
            .data;
        user.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_user_field_with_edit_context_test_cases)]
    #[case(&[SparseUserFieldWithEditContext::Id, SparseUserFieldWithEditContext::Name])]
    #[case(&[SparseUserFieldWithEditContext::Email, SparseUserFieldWithEditContext::Nickname])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_current_user_with_edit_context(
        #[case] fields: &[SparseUserFieldWithEditContext],
    ) {
        let user = api_client()
            .users()
            .filter_retrieve_me_with_edit_context(fields)
            .await
            .assert_response()
            .data;
        user.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_user_field_with_embed_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_users_with_embed_context(#[case] fields: &[SparseUserFieldWithEmbedContext]) {
        api_client()
            .users()
            .filter_list_with_embed_context(&UserListParams::default(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|user| {
                user.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_user_field_with_embed_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_user_with_embed_context(
        #[case] fields: &[SparseUserFieldWithEmbedContext],
    ) {
        let user = api_client()
            .users()
            .filter_retrieve_with_embed_context(&FIRST_USER_ID, fields)
            .await
            .assert_response()
            .data;
        user.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_user_field_with_embed_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_current_user_with_embed_context(
        #[case] fields: &[SparseUserFieldWithEmbedContext],
    ) {
        let user = api_client()
            .users()
            .filter_retrieve_me_with_embed_context(fields)
            .await
            .assert_response()
            .data;
        user.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_user_field_with_view_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_users_with_view_context(#[case] fields: &[SparseUserFieldWithViewContext]) {
        api_client()
            .users()
            .filter_list_with_view_context(&UserListParams::default(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|user| {
                user.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_user_field_with_view_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_user_with_view_context(
        #[case] fields: &[SparseUserFieldWithViewContext],
    ) {
        let user = api_client()
            .users()
            .filter_retrieve_with_view_context(&FIRST_USER_ID, fields)
            .await
            .assert_response()
            .data;
        user.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_user_field_with_view_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_current_user_with_view_context(
        #[case] fields: &[SparseUserFieldWithViewContext],
    ) {
        let user = api_client()
            .users()
            .filter_retrieve_me_with_view_context(fields)
            .await
            .assert_response()
            .data;
        user.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}
