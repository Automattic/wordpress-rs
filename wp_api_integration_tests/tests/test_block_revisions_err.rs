use wp_api::{
    block_revisions::{BlockRevisionId, BlockRevisionListParams, WpApiParamBlockRevisionsOrderBy},
    blocks::BlockId,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_parent() {
    api_client()
        .block_revisions()
        .list_with_edit_context(&BlockId(99999999), &BlockRevisionListParams::default())
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn list_err_revision_invalid_offset_number() {
    api_client()
        .block_revisions()
        .list_with_edit_context(
            &block_id(),
            &BlockRevisionListParams {
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
        .block_revisions()
        .list_with_edit_context(
            &block_id(),
            &BlockRevisionListParams {
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
        .block_revisions()
        .list_with_edit_context(&block_id(), &BlockRevisionListParams::default())
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn list_err_no_search_term_defined() {
    api_client()
        .block_revisions()
        .list_with_edit_context(
            &block_id(),
            &BlockRevisionListParams {
                orderby: Some(WpApiParamBlockRevisionsOrderBy::Relevance),
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
        .block_revisions()
        .list_with_edit_context(
            &block_id(),
            &BlockRevisionListParams {
                orderby: Some(WpApiParamBlockRevisionsOrderBy::Include),
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
        .block_revisions()
        .retrieve_with_edit_context(&BlockId(99999999), &revision_id_for_block_id())
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_invalid_id() {
    api_client()
        .block_revisions()
        .retrieve_with_edit_context(&block_id(), &BlockRevisionId(99999999))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .block_revisions()
        .retrieve_with_edit_context(&block_id(), &revision_id_for_block_id())
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn delete_err_cannot_delete_as_subscriber() {
    api_client_as_subscriber()
        .block_revisions()
        .delete(&block_id(), &revision_id_for_block_id())
        .await
        .assert_wp_error(WpErrorCode::CannotDelete)
}

#[tokio::test]
#[parallel]
async fn delete_err_post_invalid_parent() {
    api_client()
        .block_revisions()
        .delete(&BlockId(99999999), &revision_id_for_block_id())
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn delete_err_post_invalid_id() {
    api_client()
        .block_revisions()
        .delete(&block_id(), &BlockRevisionId(99999999))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId)
}

fn block_id() -> BlockId {
    BlockId(TestCredentials::instance().block_id)
}

fn revision_id_for_block_id() -> BlockRevisionId {
    BlockRevisionId(TestCredentials::instance().revision_id_for_block_id)
}
