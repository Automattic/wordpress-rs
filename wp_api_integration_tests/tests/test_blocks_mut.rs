use wp_api::blocks::BlockId;
use wp_api_integration_tests::prelude::*;

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
