use wp_api::blocks::{
    BlockCreateParams, BlockId, BlockListParams, BlockRetrieveParams, BlockUpdateParams,
    WpApiParamBlocksOrderBy,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn create_block_err_cannot_create() {
    api_client_as_subscriber()
        .blocks()
        .create(&BlockCreateParams::default())
        .await
        .assert_wp_error(WpErrorCode::CannotCreate);
}

#[tokio::test]
#[parallel]
async fn create_block_err_cannot_create2() {
    api_client_as_subscriber()
        .blocks()
        .create(&BlockCreateParams {
            title: Some("foo".to_string()),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::CannotCreate);
}

#[tokio::test]
#[parallel]
async fn delete_block_err_cannot_delete() {
    api_client_as_subscriber()
        .blocks()
        .delete(&BlockId(TestCredentials::instance().block_id))
        .await
        .assert_wp_error(WpErrorCode::CannotDelete);
}

#[tokio::test]
#[parallel]
async fn list_err_no_search_term_defined() {
    api_client()
        .blocks()
        .list_with_edit_context(&BlockListParams {
            order_by: Some(WpApiParamBlocksOrderBy::Relevance),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::NoSearchTermDefined);
}

#[tokio::test]
#[parallel]
async fn list_err_order_by_include_missing_include() {
    api_client()
        .blocks()
        .list_with_edit_context(&BlockListParams {
            order_by: Some(WpApiParamBlocksOrderBy::Include),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::OrderbyIncludeMissingInclude);
}

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_page_number() {
    api_client()
        .blocks()
        .list_with_edit_context(&BlockListParams {
            page: Some(99999999),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::PostInvalidPageNumber);
}

#[tokio::test]
#[parallel]
async fn retrieve_block_err_forbidden_context() {
    api_client_as_subscriber()
        .blocks()
        .retrieve_with_edit_context(
            &BlockId(TestCredentials::instance().block_id),
            &BlockRetrieveParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

#[tokio::test]
#[parallel]
async fn retrieve_block_err_post_invalid_id() {
    api_client()
        .blocks()
        .retrieve_with_edit_context(&BlockId(99999999), &BlockRetrieveParams::default())
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}

#[tokio::test]
#[parallel]
async fn update_block_err_cannot_edit() {
    api_client_as_author()
        .blocks()
        .update(
            &BlockId(TestCredentials::instance().block_id),
            &BlockUpdateParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotEdit);
}

#[tokio::test]
#[parallel]
async fn update_block_err_invalid_template() {
    api_client()
        .blocks()
        .update(
            &BlockId(TestCredentials::instance().block_id),
            &BlockUpdateParams {
                template: Some("foo".to_string()),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::InvalidParam);
}
