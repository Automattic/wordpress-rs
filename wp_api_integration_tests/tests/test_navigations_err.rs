use wp_api::navigations::{
    NavigationCreateParams, NavigationId, NavigationListParams, NavigationRetrieveParams,
    NavigationUpdateParams, WpApiParamNavigationsOrderBy,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn create_navigation_err_cannot_create() {
    api_client_as_subscriber()
        .navigations()
        .create(&NavigationCreateParams {
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::CannotCreate);
}

#[tokio::test]
#[parallel]
async fn create_navigation_err_cannot_create2() {
    api_client_as_subscriber()
        .navigations()
        .create(&NavigationCreateParams {
            title: Some("foo".to_string()),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::CannotCreate);
}

#[tokio::test]
#[parallel]
async fn delete_navigation_err_cannot_delete() {
    api_client_as_subscriber()
        .navigations()
        .delete(&NavigationId(TestCredentials::instance().navigation_id))
        .await
        .assert_wp_error(WpErrorCode::CannotDelete);
}

#[tokio::test]
#[parallel]
async fn list_err_no_search_term_defined() {
    api_client()
        .navigations()
        .list_with_edit_context(&NavigationListParams {
            order_by: Some(WpApiParamNavigationsOrderBy::Relevance),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::NoSearchTermDefined);
}

#[tokio::test]
#[parallel]
async fn list_err_order_by_include_missing_include() {
    api_client()
        .navigations()
        .list_with_edit_context(&NavigationListParams {
            order_by: Some(WpApiParamNavigationsOrderBy::Include),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::OrderbyIncludeMissingInclude);
}

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_page_number() {
    api_client()
        .navigations()
        .list_with_edit_context(&NavigationListParams {
            page: Some(99999999),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::PostInvalidPageNumber);
}

#[tokio::test]
#[parallel]
async fn retrieve_navigation_err_forbidden_context() {
    api_client_as_subscriber()
        .navigations()
        .retrieve_with_edit_context(
            &NavigationId(TestCredentials::instance().navigation_id),
            &NavigationRetrieveParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

#[tokio::test]
#[parallel]
async fn retrieve_navigation_err_post_invalid_id() {
    api_client()
        .navigations()
        .retrieve_with_edit_context(
            &NavigationId(99999999),
            &NavigationRetrieveParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}

#[tokio::test]
#[parallel]
async fn update_navigation_err_cannot_edit() {
    api_client_as_author()
        .navigations()
        .update(
            &NavigationId(TestCredentials::instance().navigation_id),
            &NavigationUpdateParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotEdit);
}

#[tokio::test]
#[parallel]
async fn update_navigation_err_invalid_template() {
    api_client()
        .navigations()
        .update(
            &NavigationId(TestCredentials::instance().navigation_id),
            &NavigationUpdateParams {
                template: Some("foo".to_string()),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::InvalidParam);
}
