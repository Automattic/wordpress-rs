use wp_api::{
    posts::{
        PostCreateParams, PostId, PostListParams, PostRetrieveParams, PostUpdateParams,
        WpApiParamPostsOrderBy,
    },
    request::endpoint::posts_endpoint::PostEndpointType,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn create_navigation_err_cannot_create() {
    api_client_as_subscriber()
        .posts()
        .create(
            &PostEndpointType::Navigation,
            &PostCreateParams {
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::CannotCreate);
}

#[tokio::test]
#[parallel]
async fn create_navigation_err_cannot_create2() {
    api_client_as_subscriber()
        .posts()
        .create(
            &PostEndpointType::Navigation,
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
async fn delete_navigation_err_cannot_delete() {
    api_client_as_subscriber()
        .posts()
        .delete(
            &PostEndpointType::Navigation,
            &PostId(TestCredentials::instance().navigation_id),
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
            &PostEndpointType::Navigation,
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
            &PostEndpointType::Navigation,
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
            &PostEndpointType::Navigation,
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
async fn retrieve_navigation_err_forbidden_context() {
    api_client_as_subscriber()
        .posts()
        .retrieve_with_edit_context(
            &PostEndpointType::Navigation,
            &PostId(TestCredentials::instance().navigation_id),
            &PostRetrieveParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

#[tokio::test]
#[parallel]
async fn retrieve_navigation_err_post_invalid_id() {
    api_client()
        .posts()
        .retrieve_with_edit_context(
            &PostEndpointType::Navigation,
            &PostId(99999999),
            &PostRetrieveParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}

#[tokio::test]
#[parallel]
async fn update_navigation_err_cannot_edit() {
    api_client_as_author()
        .posts()
        .update(
            &PostEndpointType::Navigation,
            &PostId(TestCredentials::instance().navigation_id),
            &PostUpdateParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotEdit);
}

#[tokio::test]
#[parallel]
async fn update_navigation_err_invalid_template() {
    api_client()
        .posts()
        .update(
            &PostEndpointType::Navigation,
            &PostId(TestCredentials::instance().navigation_id),
            &PostUpdateParams {
                template: Some("foo".to_string()),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::InvalidParam);
}
