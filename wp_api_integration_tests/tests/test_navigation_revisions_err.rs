use wp_api::{
    navigation_revisions::{
        NavigationRevisionId, NavigationRevisionListParams, WpApiParamNavigationRevisionsOrderBy,
    },
    navigations::NavigationId,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_parent() {
    api_client()
        .navigation_revisions()
        .list_with_edit_context(
            &NavigationId(99999999),
            &NavigationRevisionListParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn list_err_revision_invalid_offset_number() {
    api_client()
        .navigation_revisions()
        .list_with_edit_context(
            &navigation_id(),
            &NavigationRevisionListParams {
                offset: Some(99999999),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::RevisionInvalidOffsetNumber)
}

#[tokio::test]
#[parallel]
async fn list_err_revision_invalid_page_number() {
    api_client()
        .navigation_revisions()
        .list_with_edit_context(
            &navigation_id(),
            &NavigationRevisionListParams {
                page: Some(99999999),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::RevisionInvalidPageNumber)
}

#[tokio::test]
#[parallel]
async fn list_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .navigation_revisions()
        .list_with_edit_context(&navigation_id(), &NavigationRevisionListParams::default())
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn list_err_no_search_term_defined() {
    api_client()
        .navigation_revisions()
        .list_with_edit_context(
            &navigation_id(),
            &NavigationRevisionListParams {
                orderby: Some(WpApiParamNavigationRevisionsOrderBy::Relevance),
                search: None,
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::NoSearchTermDefined)
}

#[tokio::test]
#[parallel]
async fn list_err_orderby_include_missing_include() {
    api_client()
        .navigation_revisions()
        .list_with_edit_context(
            &navigation_id(),
            &NavigationRevisionListParams {
                orderby: Some(WpApiParamNavigationRevisionsOrderBy::Include),
                include: vec![],
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::OrderbyIncludeMissingInclude)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_invalid_parent() {
    api_client()
        .navigation_revisions()
        .retrieve_with_edit_context(&NavigationId(99999999), &revision_id_for_navigation_id())
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_invalid_id() {
    api_client()
        .navigation_revisions()
        .retrieve_with_edit_context(&navigation_id(), &NavigationRevisionId(99999999))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .navigation_revisions()
        .retrieve_with_edit_context(&navigation_id(), &revision_id_for_navigation_id())
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn delete_err_cannot_delete_as_subscriber() {
    api_client_as_subscriber()
        .navigation_revisions()
        .delete(&navigation_id(), &revision_id_for_navigation_id())
        .await
        .assert_wp_error(WpErrorCode::CannotDelete)
}

#[tokio::test]
#[parallel]
async fn delete_err_post_invalid_parent() {
    api_client()
        .navigation_revisions()
        .delete(&NavigationId(99999999), &revision_id_for_navigation_id())
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn delete_err_post_invalid_id() {
    api_client()
        .navigation_revisions()
        .delete(&navigation_id(), &NavigationRevisionId(99999999))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId)
}

fn navigation_id() -> NavigationId {
    NavigationId(TestCredentials::instance().navigation_id)
}

fn revision_id_for_navigation_id() -> NavigationRevisionId {
    NavigationRevisionId(TestCredentials::instance().revision_id_for_navigation_id)
}
