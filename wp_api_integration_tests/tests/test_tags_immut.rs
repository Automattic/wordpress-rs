use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::tags::{
    SparseTagFieldWithEditContext, SparseTagFieldWithEmbedContext, SparseTagFieldWithViewContext,
    TagListParams, WpApiParamTagsOrderBy,
};
use wp_api::{generate, WpApiParamOrder};
use wp_api_integration_tests::{api_client, AssertResponse, FIRST_POST_ID, TAG_ID_100};

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: TagListParams) {
    api_client()
        .tags()
        .list_with_edit_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: TagListParams) {
    api_client()
        .tags()
        .list_with_embed_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: TagListParams) {
    api_client()
        .tags()
        .list_with_view_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .tags()
        .retrieve_with_edit_context(&TAG_ID_100)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .tags()
        .retrieve_with_embed_context(&TAG_ID_100)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .tags()
        .retrieve_with_view_context(&TAG_ID_100)
        .await
        .assert_response();
}

#[template]
#[rstest]
#[case::default(TagListParams::default())]
#[case::page(generate!(TagListParams, (page, Some(1))))]
#[case::per_page(generate!(TagListParams, (per_page, Some(3))))]
#[case::search(generate!(TagListParams, (search, Some("foo".to_string()))))]
#[case::exclude(generate!(TagListParams, (exclude, vec![TAG_ID_100])))]
#[case::include(generate!(TagListParams, (include, vec![TAG_ID_100])))]
#[case::offset(generate!(TagListParams, (offset, Some(2))))]
#[case::order(generate!(TagListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(TagListParams, (orderby, Some(WpApiParamTagsOrderBy::Id))))]
#[case::hide_empty_false(generate!(TagListParams, (hide_empty, Some(false))))]
#[case::hide_empty_true(generate!(TagListParams, (hide_empty, Some(true))))]
#[case::post(generate!(TagListParams, (post, Some(FIRST_POST_ID))))]
#[case::slug(generate!(TagListParams, (slug, vec!["foo".to_string(), "bar".to_string()])))]
pub fn list_cases(#[case] params: TagListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_tag_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_tag_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_tag_field_with_view_context_test_cases!();

    #[apply(sparse_tag_field_with_edit_context_test_cases)]
    #[case(&[SparseTagFieldWithEditContext::Name, SparseTagFieldWithEditContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_tags_with_edit_context(
        #[case] fields: &[SparseTagFieldWithEditContext],
        #[values(
            TagListParams::default(),
            generate!(TagListParams, (orderby, Some(WpApiParamTagsOrderBy::Id))),
            generate!(TagListParams, (search, Some("foo".to_string())))
        )]
        params: TagListParams,
    ) {
        api_client()
            .tags()
            .filter_list_with_edit_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|tag| {
                tag.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_tag_field_with_edit_context_test_cases)]
    #[case(&[SparseTagFieldWithEditContext::Name, SparseTagFieldWithEditContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_posts_with_edit_context(
        #[case] fields: &[SparseTagFieldWithEditContext],
    ) {
        let tag = api_client()
            .tags()
            .filter_retrieve_with_edit_context(&TAG_ID_100, fields)
            .await
            .assert_response()
            .data;
        tag.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_tag_field_with_embed_context_test_cases)]
    #[case(&[SparseTagFieldWithEmbedContext::Name, SparseTagFieldWithEmbedContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_tags_with_embed_context(
        #[case] fields: &[SparseTagFieldWithEmbedContext],
        #[values(
            TagListParams::default(),
            generate!(TagListParams, (orderby, Some(WpApiParamTagsOrderBy::Id))),
            generate!(TagListParams, (search, Some("foo".to_string())))
        )]
        params: TagListParams,
    ) {
        api_client()
            .tags()
            .filter_list_with_embed_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|tag| {
                tag.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_tag_field_with_embed_context_test_cases)]
    #[case(&[SparseTagFieldWithEmbedContext::Name, SparseTagFieldWithEmbedContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_posts_with_embed_context(
        #[case] fields: &[SparseTagFieldWithEmbedContext],
    ) {
        let tag = api_client()
            .tags()
            .filter_retrieve_with_embed_context(&TAG_ID_100, fields)
            .await
            .assert_response()
            .data;
        tag.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_tag_field_with_view_context_test_cases)]
    #[case(&[SparseTagFieldWithViewContext::Name, SparseTagFieldWithViewContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_tags_with_view_context(
        #[case] fields: &[SparseTagFieldWithViewContext],
        #[values(
            TagListParams::default(),
            generate!(TagListParams, (orderby, Some(WpApiParamTagsOrderBy::Id))),
            generate!(TagListParams, (search, Some("foo".to_string())))
        )]
        params: TagListParams,
    ) {
        api_client()
            .tags()
            .filter_list_with_view_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|tag| {
                tag.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_tag_field_with_view_context_test_cases)]
    #[case(&[SparseTagFieldWithViewContext::Name, SparseTagFieldWithViewContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_posts_with_view_context(
        #[case] fields: &[SparseTagFieldWithViewContext],
    ) {
        let tag = api_client()
            .tags()
            .filter_retrieve_with_view_context(&TAG_ID_100, fields)
            .await
            .assert_response()
            .data;
        tag.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }
}
