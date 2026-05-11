use wp_api::block_revisions::BlockRevisionId;
use wp_api::blocks::BlockId;
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[serial]
async fn delete_block_revision() {
    let revision_id = revision_id_for_block_id();
    let block_revision_delete_response = api_client()
        .block_revisions()
        .delete(&block_id(), &revision_id)
        .await;
    assert!(
        block_revision_delete_response.is_ok(),
        "{block_revision_delete_response:#?}"
    );
    let delete_response = block_revision_delete_response.unwrap().data;
    assert!(delete_response.deleted);
    assert_eq!(delete_response.previous.id, revision_id);

    RestoreServer::db().await;
}

fn block_id() -> BlockId {
    BlockId(TestCredentials::instance().block_id)
}

fn revision_id_for_block_id() -> BlockRevisionId {
    BlockRevisionId(TestCredentials::instance().revision_id_for_block_id)
}
