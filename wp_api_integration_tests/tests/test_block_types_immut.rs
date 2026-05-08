use wp_api::block_types::{
    BlockTypeName, BlockTypeNamespace, SparseBlockTypeFieldWithEditContext,
    SparseBlockTypeFieldWithEmbedContext, SparseBlockTypeFieldWithViewContext,
};
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

mod filter {
    use super::*;

    wp_api::generate_sparse_block_type_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_block_type_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_block_type_field_with_view_context_test_cases!();

    #[apply(sparse_block_type_field_with_edit_context_test_cases)]
    #[case(&[SparseBlockTypeFieldWithEditContext::Name, SparseBlockTypeFieldWithEditContext::Title])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(#[case] fields: &[SparseBlockTypeFieldWithEditContext]) {
        api_client()
            .block_types()
            .filter_list_with_edit_context(fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|block_type| {
                block_type.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_block_type_field_with_embed_context_test_cases)]
    #[case(&[SparseBlockTypeFieldWithEmbedContext::Name, SparseBlockTypeFieldWithEmbedContext::Title])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseBlockTypeFieldWithEmbedContext],
    ) {
        api_client()
            .block_types()
            .filter_list_with_embed_context(fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|block_type| {
                block_type.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_block_type_field_with_view_context_test_cases)]
    #[case(&[SparseBlockTypeFieldWithViewContext::Name, SparseBlockTypeFieldWithViewContext::Title])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(#[case] fields: &[SparseBlockTypeFieldWithViewContext]) {
        api_client()
            .block_types()
            .filter_list_with_view_context(fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|block_type| {
                block_type.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_block_type_field_with_edit_context_test_cases)]
    #[case(&[SparseBlockTypeFieldWithEditContext::Name, SparseBlockTypeFieldWithEditContext::Title])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparseBlockTypeFieldWithEditContext],
    ) {
        api_client()
            .block_types()
            .filter_retrieve_with_edit_context(
                &BlockTypeNamespace("core".to_string()),
                &BlockTypeName("paragraph".to_string()),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_block_type_field_with_embed_context_test_cases)]
    #[case(&[SparseBlockTypeFieldWithEmbedContext::Name, SparseBlockTypeFieldWithEmbedContext::Title])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparseBlockTypeFieldWithEmbedContext],
    ) {
        api_client()
            .block_types()
            .filter_retrieve_with_embed_context(
                &BlockTypeNamespace("core".to_string()),
                &BlockTypeName("paragraph".to_string()),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_block_type_field_with_view_context_test_cases)]
    #[case(&[SparseBlockTypeFieldWithViewContext::Name, SparseBlockTypeFieldWithViewContext::Title])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparseBlockTypeFieldWithViewContext],
    ) {
        api_client()
            .block_types()
            .filter_retrieve_with_view_context(
                &BlockTypeNamespace("core".to_string()),
                &BlockTypeName("paragraph".to_string()),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}
