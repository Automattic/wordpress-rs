use rstest::*;
use rstest_reuse::{self, apply, template};
use wp_api::{
    generate, SparseUser, SparseUserField, UserListParams, WPApiParamOrder, WPApiParamUsersOrderBy,
    WPApiParamUsersWho, WPContext,
};

use crate::test_helpers::{
    api, WPNetworkRequestExecutor, WPNetworkResponseParser, FIRST_USER_ID, SECOND_USER_ID,
};

pub mod test_helpers;

#[apply(filter_fields_cases)]
#[tokio::test]
async fn filter_users(#[case] fields: &[SparseUserField]) {
    let parsed_response = api()
        .filter_list_users_request(WPContext::Edit, &None, fields)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_filter_users_response);
    assert!(parsed_response.is_ok());
    parsed_response
        .unwrap()
        .iter()
        .for_each(|user| validate_sparse_user_fields(&user, fields));
}

#[apply(filter_fields_cases)]
#[tokio::test]
async fn filter_retrieve_user(#[case] fields: &[SparseUserField]) {
    let user_result = api()
        .filter_retrieve_user_request(FIRST_USER_ID, WPContext::Edit, fields)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_filter_retrieve_user_response);
    assert!(user_result.is_ok());
    validate_sparse_user_fields(&user_result.unwrap(), fields);
}

#[apply(filter_fields_cases)]
#[tokio::test]
async fn filter_retrieve_current_user(#[case] fields: &[SparseUserField]) {
    let user_result = api()
        .filter_retrieve_current_user_request(WPContext::Edit, fields)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_filter_retrieve_user_response);
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
#[case(generate!(UserListParams, (order, Some(WPApiParamOrder::Asc))))]
#[case(generate!(UserListParams, (orderby, Some(WPApiParamUsersOrderBy::Id))))]
#[case(generate!(UserListParams, (order, Some(WPApiParamOrder::Desc)), (orderby, Some(WPApiParamUsersOrderBy::Email))))]
#[case(generate!(UserListParams, (slug, vec!["foo".to_string(), "bar".to_string()])))]
#[case(generate!(UserListParams, (roles, vec!["author".to_string(), "editor".to_string()])))]
#[case(generate!(UserListParams, (slug, vec!["foo".to_string(), "bar".to_string()]), (roles, vec!["author".to_string(), "editor".to_string()])))]
#[case(generate!(UserListParams, (capabilities, vec!["edit_themes".to_string(), "delete_pages".to_string()])))]
#[case::who_all_param_should_be_empty(generate!(UserListParams, (who, Some(WPApiParamUsersWho::All))))]
#[case(generate!(UserListParams, (who, Some(WPApiParamUsersWho::Authors))))]
#[case(generate!(UserListParams, (has_published_posts, Some(true))))]
#[trace]
#[tokio::test]
async fn test_user_list_params_parametrized(
    #[case] params: UserListParams,
    #[values(WPContext::Edit, WPContext::Embed, WPContext::View)] context: WPContext,
) {
    let response = api()
        .list_users_request(context, &Some(params))
        .execute()
        .await
        .unwrap();
    match context {
        WPContext::Edit => {
            let parsed_response = wp_api::parse_list_users_response_with_edit_context(&response);
            assert!(
                parsed_response.is_ok(),
                "Response was: '{:?}'",
                parsed_response
            );
        }
        WPContext::Embed => {
            let parsed_response = wp_api::parse_list_users_response_with_embed_context(&response);
            assert!(
                parsed_response.is_ok(),
                "Response was: '{:?}'",
                parsed_response
            );
        }
        WPContext::View => {
            let parsed_response = wp_api::parse_list_users_response_with_view_context(&response);
            assert!(
                parsed_response.is_ok(),
                "Response was: '{:?}'",
                parsed_response
            );
        }
    };
}

#[tokio::test]
async fn retrieve_user_with_edit_context() {
    assert!(api()
        .retrieve_user_request(FIRST_USER_ID, WPContext::Edit)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .is_ok());
}

#[tokio::test]
async fn retrieve_user_with_embed_context() {
    assert!(api()
        .retrieve_user_request(FIRST_USER_ID, WPContext::Embed)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_embed_context)
        .is_ok());
}

#[tokio::test]
async fn retrieve_user_with_view_context() {
    assert!(api()
        .retrieve_user_request(FIRST_USER_ID, WPContext::View)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_view_context)
        .is_ok());
}

#[tokio::test]
async fn retrieve_current_user_with_edit_context() {
    assert!(api()
        .retrieve_current_user_request(WPContext::Edit)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .is_ok());
}

#[tokio::test]
async fn retrieve_current_user_with_embed_context() {
    assert!(api()
        .retrieve_current_user_request(WPContext::Embed)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_embed_context)
        .is_ok());
}

#[tokio::test]
async fn retrieve_current_user_with_view_context() {
    assert!(api()
        .retrieve_current_user_request(WPContext::View)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_view_context)
        .is_ok());
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
