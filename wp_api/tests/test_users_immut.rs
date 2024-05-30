use rstest::*;
use rstest_reuse::{self, apply, template};
use wp_api::{
    generate,
    request::WpNetworkResponse,
    users::{
        SparseUser, SparseUserField, UserId, UserListParams, WpApiParamUsersHasPublishedPosts,
        WpApiParamUsersOrderBy, WpApiParamUsersWho,
    },
    WpApiParamOrder, WpContext,
};

use crate::integration_test_common::{
    request_builder, WpNetworkRequestExecutor, FIRST_USER_ID, SECOND_USER_ID,
};

pub mod integration_test_common;

#[apply(filter_fields_cases)]
#[tokio::test]
async fn filter_users(#[case] fields: &[SparseUserField]) {
    let parsed_response = request_builder()
        .users()
        .filter_list(WpContext::Edit, &None, fields)
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_filter_users_response);
    assert!(parsed_response.is_ok());
    parsed_response
        .unwrap()
        .iter()
        .for_each(|user| validate_sparse_user_fields(user, fields));
}

#[apply(filter_fields_cases)]
#[tokio::test]
async fn filter_retrieve_user(#[case] fields: &[SparseUserField]) {
    let user_result = request_builder()
        .users()
        .filter_retrieve(FIRST_USER_ID, WpContext::Edit, fields)
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_filter_retrieve_user_response);
    assert!(user_result.is_ok());
    validate_sparse_user_fields(&user_result.unwrap(), fields);
}

#[apply(filter_fields_cases)]
#[tokio::test]
async fn filter_retrieve_current_user(#[case] fields: &[SparseUserField]) {
    let user_result = request_builder()
        .users()
        .filter_retrieve_me(WpContext::Edit, fields)
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::users::parse_filter_retrieve_user_response);
    assert!(user_result.is_ok());
    validate_sparse_user_fields(&user_result.unwrap(), fields);
}

#[rstest]
#[case(UserListParams::default())]
#[case(generate!(UserListParams, (page, Some(1))))]
#[case(generate!(UserListParams, (page, Some(2)), (per_page, Some(5))))]
#[case(generate!(UserListParams, (search, Some("foo".to_string()))))]
#[case(generate!(UserListParams, (exclude, vec![FIRST_USER_ID, SECOND_USER_ID])))]
#[case(generate!(UserListParams, (include, vec![FIRST_USER_ID])))]
#[case(generate!(UserListParams, (per_page, Some(100)), (offset, Some(20))))]
#[case(generate!(UserListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case(generate!(UserListParams, (orderby, Some(WpApiParamUsersOrderBy::Id))))]
#[case(generate!(UserListParams, (order, Some(WpApiParamOrder::Desc)), (orderby, Some(WpApiParamUsersOrderBy::Email))))]
#[case(generate!(UserListParams, (slug, vec!["foo".to_string(), "bar".to_string()])))]
#[case(generate!(UserListParams, (roles, vec!["author".to_string(), "editor".to_string()])))]
#[case(generate!(UserListParams, (slug, vec!["foo".to_string(), "bar".to_string()]), (roles, vec!["author".to_string(), "editor".to_string()])))]
#[case(generate!(UserListParams, (capabilities, vec!["edit_themes".to_string(), "delete_pages".to_string()])))]
#[case::who_all_param_should_be_empty(generate!(UserListParams, (who, Some(WpApiParamUsersWho::All))))]
#[case(generate!(UserListParams, (who, Some(WpApiParamUsersWho::Authors))))]
#[case(generate!(UserListParams, (has_published_posts, Some(WpApiParamUsersHasPublishedPosts::True))))]
#[trace]
#[tokio::test]
async fn list_users_parametrized(
    #[case] params: UserListParams,
    #[values(WpContext::Edit, WpContext::Embed, WpContext::View)] context: WpContext,
) {
    let response = request_builder()
        .users()
        .list(context, &Some(params))
        .execute()
        .await
        .unwrap();
    match context {
        WpContext::Edit => {
            let parsed_response =
                wp_api::users::parse_list_users_response_with_edit_context(&response);
            assert!(
                parsed_response.is_ok(),
                "Response was: '{:?}'",
                parsed_response
            );
        }
        WpContext::Embed => {
            let parsed_response =
                wp_api::users::parse_list_users_response_with_embed_context(&response);
            assert!(
                parsed_response.is_ok(),
                "Response was: '{:?}'",
                parsed_response
            );
        }
        WpContext::View => {
            let parsed_response =
                wp_api::users::parse_list_users_response_with_view_context(&response);
            assert!(
                parsed_response.is_ok(),
                "Response was: '{:?}'",
                parsed_response
            );
        }
    };
}

#[rstest]
#[case(None)]
#[case(Some(WpApiParamUsersHasPublishedPosts::True))]
#[case(Some(WpApiParamUsersHasPublishedPosts::False))]
#[case(Some(WpApiParamUsersHasPublishedPosts::PostTypes(vec!["post".to_string()])))]
#[case(Some(WpApiParamUsersHasPublishedPosts::PostTypes(vec!["post".to_string(), "page".to_string()])))]
#[trace]
#[tokio::test]
async fn list_users_has_published_posts(
    #[case] has_published_posts: Option<WpApiParamUsersHasPublishedPosts>,
    #[values(WpContext::Edit, WpContext::Embed, WpContext::View)] context: WpContext,
) {
    let response = request_builder()
        .users()
        .list(
            context,
            &Some(UserListParams {
                has_published_posts,
                ..Default::default()
            }),
        )
        .execute()
        .await;
    assert!(response.is_ok());
    let response = &response.unwrap();
    match context {
        WpContext::Edit => {
            assert!(wp_api::users::parse_list_users_response_with_edit_context(response).is_ok())
        }
        WpContext::Embed => {
            assert!(wp_api::users::parse_list_users_response_with_embed_context(response).is_ok())
        }
        WpContext::View => {
            assert!(wp_api::users::parse_list_users_response_with_view_context(response).is_ok())
        }
    };
}

#[rstest]
#[trace]
#[tokio::test]
async fn retrieve_user(
    #[values(FIRST_USER_ID, SECOND_USER_ID)] user_id: UserId,
    #[values(WpContext::Edit, WpContext::Embed, WpContext::View)] context: WpContext,
) {
    let response = request_builder()
        .users()
        .retrieve(user_id, context)
        .execute()
        .await;
    assert!(response.is_ok());
    assert_retrieve_user_response(&response.unwrap(), context, user_id);
}

#[rstest]
#[trace]
#[tokio::test]
async fn retrieve_me(
    #[values(WpContext::Edit, WpContext::Embed, WpContext::View)] context: WpContext,
) {
    let response = request_builder()
        .users()
        .retrieve_me(context)
        .execute()
        .await;
    assert!(response.is_ok());
    // FIRST_USER_ID is the current user's id
    assert_retrieve_user_response(&response.unwrap(), context, FIRST_USER_ID);
}

fn assert_retrieve_user_response(
    response: &WpNetworkResponse,
    context: WpContext,
    expected_user_id: UserId,
) {
    let user_id = match context {
        WpContext::Edit => {
            let parsed_response =
                wp_api::users::parse_retrieve_user_response_with_edit_context(response);
            assert!(parsed_response.is_ok());
            parsed_response.unwrap().id
        }
        WpContext::Embed => {
            let parsed_response =
                wp_api::users::parse_retrieve_user_response_with_embed_context(response);
            assert!(parsed_response.is_ok());
            parsed_response.unwrap().id
        }
        WpContext::View => {
            let parsed_response =
                wp_api::users::parse_retrieve_user_response_with_view_context(response);
            assert!(parsed_response.is_ok());
            parsed_response.unwrap().id
        }
    };
    assert_eq!(expected_user_id, user_id);
}

fn validate_sparse_user_fields(user: &SparseUser, fields: &[SparseUserField]) {
    assert_eq!(user.id.is_some(), fields.contains(&SparseUserField::Id));
    assert_eq!(
        user.username.is_some(),
        fields.contains(&SparseUserField::Username)
    );
    assert_eq!(user.name.is_some(), fields.contains(&SparseUserField::Name));
    assert_eq!(
        user.last_name.is_some(),
        fields.contains(&SparseUserField::LastName)
    );
    assert_eq!(
        user.email.is_some(),
        fields.contains(&SparseUserField::Email)
    );
    assert_eq!(user.url.is_some(), fields.contains(&SparseUserField::Url));
    assert_eq!(
        user.description.is_some(),
        fields.contains(&SparseUserField::Description)
    );
    assert_eq!(user.link.is_some(), fields.contains(&SparseUserField::Link));
    assert_eq!(
        user.locale.is_some(),
        fields.contains(&SparseUserField::Locale)
    );
    assert_eq!(
        user.nickname.is_some(),
        fields.contains(&SparseUserField::Nickname)
    );
    assert_eq!(user.slug.is_some(), fields.contains(&SparseUserField::Slug));
    assert_eq!(
        user.registered_date.is_some(),
        fields.contains(&SparseUserField::RegisteredDate)
    );
    assert_eq!(
        user.roles.is_some(),
        fields.contains(&SparseUserField::Roles)
    );
    assert_eq!(
        user.capabilities.is_some(),
        fields.contains(&SparseUserField::Capabilities)
    );
    assert_eq!(
        user.extra_capabilities.is_some(),
        fields.contains(&SparseUserField::ExtraCapabilities)
    );
    assert_eq!(
        user.avatar_urls.is_some(),
        fields.contains(&SparseUserField::AvatarUrls)
    );
}

#[template]
#[rstest]
#[case(&[SparseUserField::Id])]
#[case(&[SparseUserField::Username])]
#[case(&[SparseUserField::Name])]
#[case(&[SparseUserField::LastName])]
#[case(&[SparseUserField::Email])]
#[case(&[SparseUserField::Url])]
#[case(&[SparseUserField::Description])]
#[case(&[SparseUserField::Link])]
#[case(&[SparseUserField::Locale])]
#[case(&[SparseUserField::Nickname])]
#[case(&[SparseUserField::Slug])]
#[case(&[SparseUserField::RegisteredDate])]
#[case(&[SparseUserField::Roles])]
#[case(&[SparseUserField::Capabilities])]
#[case(&[SparseUserField::ExtraCapabilities])]
#[case(&[SparseUserField::AvatarUrls])]
#[case(&[SparseUserField::Id, SparseUserField::Name])]
#[case(&[SparseUserField::Email, SparseUserField::Nickname])]
fn filter_fields_cases(#[case] fields: &[SparseUserField]) {}
