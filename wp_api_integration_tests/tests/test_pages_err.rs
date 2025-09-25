use wp_api::request::endpoint::posts_endpoint::PostEndpointType;
use wp_api::{
    posts::{
        PostCreateParams, PostId, PostListParams, PostRetrieveParams, PostUpdateParams,
        WpApiParamPostsOrderBy,
    },
    users::UserId,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn create_page_err_cannot_create() {
    api_client_as_subscriber()
        .posts()
        .create(
            &PostEndpointType::Pages,
            &PostCreateParams {
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::CannotCreate);
}

#[tokio::test]
#[parallel]
async fn create_page_err_cannot_create2() {
    api_client_as_subscriber()
        .posts()
        .create(
            &PostEndpointType::Pages,
            &PostCreateParams {
                title: Some("foo".to_string()),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::CannotCreate);
}

#[tokio::test]
#[parallel]
async fn delete_page_err_cannot_delete() {
    api_client_as_subscriber()
        .posts()
        .delete(
            &PostEndpointType::Pages,
            &PostId(TestCredentials::instance().first_page_id),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotDelete);
}

#[tokio::test]
#[parallel]
async fn list_err_no_search_term_defined() {
    api_client()
        .posts()
        .list_with_edit_context(
            &PostEndpointType::Pages,
            &PostListParams {
                orderby: Some(WpApiParamPostsOrderBy::Relevance),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::NoSearchTermDefined);
}

#[tokio::test]
#[parallel]
async fn list_err_order_by_include_missing_include() {
    api_client()
        .posts()
        .list_with_edit_context(
            &PostEndpointType::Pages,
            &PostListParams {
                orderby: Some(WpApiParamPostsOrderBy::Include),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::OrderbyIncludeMissingInclude);
}

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_page_number() {
    api_client()
        .posts()
        .list_with_edit_context(
            &PostEndpointType::Pages,
            &PostListParams {
                page: Some(99999999),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidPageNumber);
}

#[tokio::test]
#[parallel]
async fn retrieve_password_protected_page_err_wrong_password() {
    api_client()
        .posts()
        .retrieve_with_view_context(
            &PostEndpointType::Pages,
            &PostId(TestCredentials::instance().password_protected_page_id),
            &PostRetrieveParams {
                password: Some("wrong_password".to_string()),
            },
        )
        .await
        .assert_wp_error(WpErrorCode::PostIncorrectPassword);
}

#[tokio::test]
#[parallel]
async fn retrieve_page_err_forbidden_context() {
    api_client_as_subscriber()
        .posts()
        .retrieve_with_edit_context(
            &PostEndpointType::Pages,
            &PostId(TestCredentials::instance().first_page_id),
            &PostRetrieveParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

#[tokio::test]
#[parallel]
async fn retrieve_page_err_post_invalid_id() {
    api_client()
        .posts()
        .retrieve_with_edit_context(
            &PostEndpointType::Pages,
            &PostId(99999999),
            &PostRetrieveParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}

#[tokio::test]
#[parallel]
async fn trash_page_err_already_trashed() {
    api_client()
        .posts()
        .trash(
            &PostEndpointType::Pages,
            &PostId(TestCredentials::instance().trashed_page_id),
        )
        .await
        .assert_wp_error(WpErrorCode::AlreadyTrashed);
}

#[tokio::test]
#[parallel]
async fn update_page_err_cannot_edit() {
    api_client_as_author()
        .posts()
        .update(
            &PostEndpointType::Pages,
            &PostId(TestCredentials::instance().first_page_id),
            &PostUpdateParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotEdit);
}

#[tokio::test]
#[parallel]
async fn update_page_err_invalid_author() {
    api_client()
        .posts()
        .update(
            &PostEndpointType::Pages,
            &PostId(TestCredentials::instance().first_page_id),
            &PostUpdateParams {
                author: Some(UserId(99999999)),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::InvalidAuthor);
}

#[tokio::test]
#[parallel]
async fn update_page_err_invalid_parent() {
    api_client()
        .posts()
        .update(
            &PostEndpointType::Pages,
            &PostId(TestCredentials::instance().first_page_id),
            &PostUpdateParams {
                parent: Some(PostId(99999999)),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}

#[tokio::test]
#[parallel]
async fn update_page_err_invalid_param() {
    api_client()
        .posts()
        .update(
            &PostEndpointType::Pages,
            &PostId(TestCredentials::instance().first_page_id),
            &PostUpdateParams {
                template: Some("nonexistent-template".to_string()),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::InvalidParam);
}
