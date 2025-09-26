use wp_api::{
    post_revisions::{AnyPostRevisionListParams, PostRevisionId, WpApiParamPostRevisionsOrderBy},
    posts::PostId,
    request::endpoint::posts_endpoint::PostEndpointType,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_parent() {
    api_client()
        .post_revisions()
        .list_with_edit_context(
            &PostEndpointType::Pages,
            &PostId(99999999),
            &AnyPostRevisionListParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn list_err_revision_invalid_offset_number() {
    api_client()
        .post_revisions()
        .list_with_edit_context(
            &PostEndpointType::Pages,
            &revisioned_page_id(),
            &AnyPostRevisionListParams {
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
        .post_revisions()
        .list_with_edit_context(
            &PostEndpointType::Pages,
            &revisioned_page_id(),
            &AnyPostRevisionListParams {
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
        .post_revisions()
        .list_with_edit_context(
            &PostEndpointType::Pages,
            &revisioned_page_id(),
            &AnyPostRevisionListParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn list_err_no_search_term_defined() {
    api_client()
        .post_revisions()
        .list_with_edit_context(
            &PostEndpointType::Pages,
            &revisioned_page_id(),
            &AnyPostRevisionListParams {
                orderby: Some(WpApiParamPostRevisionsOrderBy::Relevance),
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
        .post_revisions()
        .list_with_edit_context(
            &PostEndpointType::Pages,
            &revisioned_page_id(),
            &AnyPostRevisionListParams {
                orderby: Some(WpApiParamPostRevisionsOrderBy::Include),
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
        .post_revisions()
        .retrieve_with_edit_context(
            &PostEndpointType::Pages,
            &PostId(99999999),
            &valid_page_revision_id(),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_invalid_id() {
    api_client()
        .post_revisions()
        .retrieve_with_edit_context(
            &PostEndpointType::Pages,
            &revisioned_page_id(),
            &PostRevisionId(99999999),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .post_revisions()
        .retrieve_with_edit_context(
            &PostEndpointType::Pages,
            &revisioned_page_id(),
            &valid_page_revision_id(),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn delete_err_cannot_delete_as_subscriber() {
    api_client_as_subscriber()
        .post_revisions()
        .delete(
            &PostEndpointType::Pages,
            &revisioned_page_id(),
            &valid_page_revision_id(),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotDelete)
}

#[tokio::test]
#[parallel]
async fn delete_err_post_invalid_parent() {
    api_client()
        .post_revisions()
        .delete(
            &PostEndpointType::Pages,
            &PostId(99999999),
            &valid_page_revision_id(),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn delete_err_post_invalid_id() {
    api_client()
        .post_revisions()
        .delete(
            &PostEndpointType::Pages,
            &revisioned_page_id(),
            &PostRevisionId(99999999),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId)
}

fn revisioned_page_id() -> PostId {
    PostId(TestCredentials::instance().revisioned_page_id)
}

fn valid_page_revision_id() -> PostRevisionId {
    PostRevisionId(TestCredentials::instance().revision_id_for_revisioned_page_id)
}
