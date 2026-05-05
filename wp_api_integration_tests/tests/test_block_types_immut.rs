use wp_api::block_types::{BlockTypeName, BlockTypeNamespace};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_with_edit_context() {
    let block_types = api_client()
        .block_types()
        .list_with_edit_context()
        .await
        .assert_response()
        .data;
    assert!(!block_types.is_empty());
}

#[tokio::test]
#[parallel]
async fn list_with_embed_context() {
    api_client()
        .block_types()
        .list_with_embed_context()
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_view_context() {
    api_client()
        .block_types()
        .list_with_view_context()
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_by_namespace_with_edit_context() {
    let block_types = api_client()
        .block_types()
        .list_by_namespace_with_edit_context(&BlockTypeNamespace("core".to_string()))
        .await
        .assert_response()
        .data;
    assert!(!block_types.is_empty());
}

#[tokio::test]
#[parallel]
async fn list_by_namespace_with_embed_context() {
    api_client()
        .block_types()
        .list_by_namespace_with_embed_context(&BlockTypeNamespace("core".to_string()))
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_by_namespace_with_view_context() {
    api_client()
        .block_types()
        .list_by_namespace_with_view_context(&BlockTypeNamespace("core".to_string()))
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    let block_type = api_client()
        .block_types()
        .retrieve_with_edit_context(
            &BlockTypeNamespace("core".to_string()),
            &BlockTypeName("paragraph".to_string()),
        )
        .await
        .assert_response()
        .data;
    assert_eq!(block_type.name, "core/paragraph");
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    let block_type = api_client()
        .block_types()
        .retrieve_with_embed_context(
            &BlockTypeNamespace("core".to_string()),
            &BlockTypeName("paragraph".to_string()),
        )
        .await
        .assert_response()
        .data;
    assert_eq!(block_type.name, "core/paragraph");
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    let block_type = api_client()
        .block_types()
        .retrieve_with_view_context(
            &BlockTypeNamespace("core".to_string()),
            &BlockTypeName("paragraph".to_string()),
        )
        .await
        .assert_response()
        .data;
    assert_eq!(block_type.name, "core/paragraph");
}
