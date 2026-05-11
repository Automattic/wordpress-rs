use wp_api::blocks::{BlockCreateParams, BlockId, BlockWithEditContext};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[serial]
async fn create_block_with_just_title() {
    test_create_block(
        &BlockCreateParams {
            title: Some("foo".to_string()),
            ..Default::default()
        },
        |created_block| {
            assert_eq!(created_block.title.raw, Some("foo".to_string()));
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_block_with_just_content() {
    test_create_block(
        &BlockCreateParams {
            content: Some("foo".to_string()),
            ..Default::default()
        },
        |created_block| {
            assert_eq!(created_block.content.raw, Some("foo".to_string()));
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_block_with_title_and_content() {
    test_create_block(
        &BlockCreateParams {
            title: Some("foo".to_string()),
            content: Some("bar".to_string()),
            ..Default::default()
        },
        |created_block| {
            assert_eq!(created_block.title.raw, Some("foo".to_string()));
            assert_eq!(created_block.content.raw, Some("bar".to_string()));
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn delete_block() {
    let block_delete_response = api_client()
        .blocks()
        .delete(&BlockId(TestCredentials::instance().block_id))
        .await;
    assert!(block_delete_response.is_ok(), "{block_delete_response:#?}");
    assert!(block_delete_response.unwrap().data.deleted);

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn trash_block() {
    let block_trash_response = api_client()
        .blocks()
        .trash(&BlockId(TestCredentials::instance().block_id))
        .await;
    assert!(block_trash_response.is_ok(), "{block_trash_response:#?}");

    RestoreServer::db().await;
}

async fn test_create_block<F>(params: &BlockCreateParams, assert: F)
where
    F: Fn(BlockWithEditContext),
{
    let created_block = api_client()
        .blocks()
        .create(params)
        .await
        .assert_response()
        .data;
    assert(created_block);
    RestoreServer::db().await;
}
