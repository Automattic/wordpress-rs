use wp_api::blocks::{
    BlockId, BlockListParams, BlockStatus, SparseBlockFieldWithEditContext,
    SparseBlockFieldWithEmbedContext, SparseBlockFieldWithViewContext, WpApiParamBlocksOrderBy,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: BlockListParams) {
    api_client()
        .blocks()
        .list_with_edit_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: BlockListParams) {
    api_client()
        .blocks()
        .list_with_embed_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: BlockListParams) {
    api_client()
        .blocks()
        .list_with_view_context(&params)
        .await
        .assert_response();
}

#[template]
#[rstest]
#[case::default(BlockListParams::default())]
#[case::page(generate!(BlockListParams, (page, Some(1))))]
#[case::per_page(generate!(BlockListParams, (per_page, Some(3))))]
#[case::search(generate!(BlockListParams, (search, Some("foo".to_string()))))]
#[case::after(generate!(BlockListParams, (after, Some(unwrapped_wp_gmt_date_time("2020-08-14T17:00:00+0200")))))]
#[case::modified_after(generate!(BlockListParams, (modified_after, Some(unwrapped_wp_gmt_date_time("2024-01-14T17:00:00+0200")))))]
#[case::before(generate!(BlockListParams, (before, Some(unwrapped_wp_gmt_date_time("2023-08-14T17:00:00+0200")))))]
#[case::modified_before(generate!(BlockListParams, (modified_before, Some(unwrapped_wp_gmt_date_time("2024-01-14T17:00:00+0200")))))]
#[case::exclude(generate!(BlockListParams, (exclude, vec![BlockId(1), BlockId(2)])))]
#[case::include(generate!(BlockListParams, (include, vec![BlockId(1)])))]
#[case::offset(generate!(BlockListParams, (offset, Some(2))))]
#[case::order(generate!(BlockListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(BlockListParams, (order_by, Some(WpApiParamBlocksOrderBy::Id))))]
#[case::search_columns(generate!(BlockListParams, (search_columns, vec!["post_content".to_string(), "post_excerpt".to_string()])))]
#[case::slug(generate!(BlockListParams, (slug, vec!["foo".to_string(), "bar".to_string()])))]
#[case::status(generate!(BlockListParams, (status, vec![BlockStatus::Publish, BlockStatus::Draft])))]
fn list_cases(#[case] params: BlockListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_block_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_block_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_block_field_with_view_context_test_cases!();

    #[apply(sparse_block_field_with_edit_context_test_cases)]
    #[case(&[SparseBlockFieldWithEditContext::Id, SparseBlockFieldWithEditContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_blocks_with_edit_context(
        #[case] fields: &[SparseBlockFieldWithEditContext],
        #[values(
            BlockListParams::default(),
            generate!(BlockListParams, (status, vec![BlockStatus::Draft, BlockStatus::Publish])),
            generate!(BlockListParams, (search, Some("foo".to_string())))
        )]
        params: BlockListParams,
    ) {
        api_client()
            .blocks()
            .filter_list_with_edit_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|block| {
                block.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_block_field_with_embed_context_test_cases)]
    #[case(&[SparseBlockFieldWithEmbedContext::Id, SparseBlockFieldWithEmbedContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_blocks_with_embed_context(
        #[case] fields: &[SparseBlockFieldWithEmbedContext],
        #[values(
            BlockListParams::default(),
            generate!(BlockListParams, (status, vec![BlockStatus::Draft, BlockStatus::Publish])),
            generate!(BlockListParams, (search, Some("foo".to_string())))
        )]
        params: BlockListParams,
    ) {
        api_client()
            .blocks()
            .filter_list_with_embed_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|block| {
                block.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_block_field_with_view_context_test_cases)]
    #[case(&[SparseBlockFieldWithViewContext::Id, SparseBlockFieldWithViewContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_blocks_with_view_context(
        #[case] fields: &[SparseBlockFieldWithViewContext],
        #[values(
            BlockListParams::default(),
            generate!(BlockListParams, (status, vec![BlockStatus::Draft, BlockStatus::Publish])),
            generate!(BlockListParams, (search, Some("foo".to_string())))
        )]
        params: BlockListParams,
    ) {
        api_client()
            .blocks()
            .filter_list_with_view_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|block| {
                block.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }
}
