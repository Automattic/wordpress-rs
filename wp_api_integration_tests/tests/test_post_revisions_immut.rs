use wp_api::{
    post_revisions::{
        PostRevisionId, PostRevisionListParams, SparsePostRevisionFieldWithEditContext,
        SparsePostRevisionFieldWithEmbedContext, SparsePostRevisionFieldWithViewContext,
        WpApiParamPostRevisionsOrderBy,
    },
    posts::PostId,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: PostRevisionListParams) {
    api_client()
        .post_revisions()
        .list_with_edit_context(&revisioned_post_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: PostRevisionListParams) {
    api_client()
        .post_revisions()
        .list_with_embed_context(&revisioned_post_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: PostRevisionListParams) {
    api_client()
        .post_revisions()
        .list_with_view_context(&revisioned_post_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .post_revisions()
        .retrieve_with_edit_context(&revisioned_post_id(), &revision_id_for_revisioned_post_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .post_revisions()
        .retrieve_with_embed_context(&revisioned_post_id(), &revision_id_for_revisioned_post_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .post_revisions()
        .retrieve_with_view_context(&revisioned_post_id(), &revision_id_for_revisioned_post_id())
        .await
        .assert_response();
}

fn revisioned_post_id() -> PostId {
    PostId(TestCredentials::instance().revisioned_post_id)
}

fn revision_id_for_revisioned_post_id() -> PostRevisionId {
    PostRevisionId(TestCredentials::instance().revision_id_for_revisioned_post_id)
}

#[template]
#[rstest]
#[case::default(PostRevisionListParams::default())]
#[case::page(generate!(PostRevisionListParams, (page, Some(1))))]
#[case::per_page(generate!(PostRevisionListParams, (per_page, Some(3))))]
#[case::search(generate!(PostRevisionListParams, (search, Some("foo".to_string()))))]
#[case::exclude(generate!(PostRevisionListParams, (exclude, vec![PostRevisionId(1), PostRevisionId(2)])))]
#[case::include(generate!(PostRevisionListParams, (include, vec![PostRevisionId(1)])))]
#[case::offset(generate!(PostRevisionListParams, (offset, Some(5))))]
#[case::order(generate!(PostRevisionListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(PostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Slug))))]
fn list_cases(#[case] params: PostRevisionListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_post_revision_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_post_revision_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_post_revision_field_with_view_context_test_cases!();

    #[apply(sparse_post_revision_field_with_edit_context_test_cases)]
    #[case(&[SparsePostRevisionFieldWithEditContext::Id, SparsePostRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparsePostRevisionFieldWithEditContext],
        #[values(
            PostRevisionListParams::default(),
            generate!(PostRevisionListParams, (exclude, vec![PostRevisionId(2), PostRevisionId(3)])),
            generate!(PostRevisionListParams, (search, Some("foo".to_string())))
        )]
        params: PostRevisionListParams,
    ) {
        api_client()
            .post_revisions()
            .filter_list_with_edit_context(&revisioned_post_id(), &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|post| {
                post.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_post_revision_field_with_embed_context_test_cases)]
    #[case(&[SparsePostRevisionFieldWithEmbedContext::Id, SparsePostRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparsePostRevisionFieldWithEmbedContext],
        #[values(
            PostRevisionListParams::default(),
            generate!(PostRevisionListParams, (exclude, vec![PostRevisionId(2), PostRevisionId(3)])),
            generate!(PostRevisionListParams, (search, Some("foo".to_string())))
        )]
        params: PostRevisionListParams,
    ) {
        api_client()
            .post_revisions()
            .filter_list_with_embed_context(&revisioned_post_id(), &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|post| {
                post.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_post_revision_field_with_view_context_test_cases)]
    #[case(&[SparsePostRevisionFieldWithViewContext::Id, SparsePostRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparsePostRevisionFieldWithViewContext],
        #[values(
            PostRevisionListParams::default(),
            generate!(PostRevisionListParams, (exclude, vec![PostRevisionId(2), PostRevisionId(3)])),
            generate!(PostRevisionListParams, (search, Some("foo".to_string())))
        )]
        params: PostRevisionListParams,
    ) {
        api_client()
            .post_revisions()
            .filter_list_with_view_context(&revisioned_post_id(), &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|post| {
                post.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_post_revision_field_with_edit_context_test_cases)]
    #[case(&[SparsePostRevisionFieldWithEditContext::Id, SparsePostRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparsePostRevisionFieldWithEditContext],
    ) {
        api_client()
            .post_revisions()
            .filter_retrieve_with_edit_context(
                &revisioned_post_id(),
                &revision_id_for_revisioned_post_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_post_revision_field_with_embed_context_test_cases)]
    #[case(&[SparsePostRevisionFieldWithEmbedContext::Id, SparsePostRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparsePostRevisionFieldWithEmbedContext],
    ) {
        api_client()
            .post_revisions()
            .filter_retrieve_with_embed_context(
                &revisioned_post_id(),
                &revision_id_for_revisioned_post_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_post_revision_field_with_view_context_test_cases)]
    #[case(&[SparsePostRevisionFieldWithViewContext::Id, SparsePostRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparsePostRevisionFieldWithViewContext],
    ) {
        api_client()
            .post_revisions()
            .filter_retrieve_with_view_context(
                &revisioned_post_id(),
                &revision_id_for_revisioned_post_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}
