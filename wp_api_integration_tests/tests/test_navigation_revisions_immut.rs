use wp_api::{
    post_revisions::{
        AnyPostRevisionListParams, PostRevisionId, SparseAnyPostRevisionFieldWithEditContext,
        SparseAnyPostRevisionFieldWithEmbedContext, SparseAnyPostRevisionFieldWithViewContext,
        WpApiParamPostRevisionsOrderBy,
    },
    request::endpoint::posts_endpoint::PostEndpointType,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: AnyPostRevisionListParams) {
    api_client()
        .post_revisions()
        .list_with_edit_context(&PostEndpointType::Navigation, &navigation_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: AnyPostRevisionListParams) {
    api_client()
        .post_revisions()
        .list_with_embed_context(&PostEndpointType::Navigation, &navigation_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: AnyPostRevisionListParams) {
    api_client()
        .post_revisions()
        .list_with_view_context(&PostEndpointType::Navigation, &navigation_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .post_revisions()
        .retrieve_with_edit_context(
            &PostEndpointType::Navigation,
            &navigation_id(),
            &revision_id_for_navigation_id(),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .post_revisions()
        .retrieve_with_embed_context(
            &PostEndpointType::Navigation,
            &navigation_id(),
            &revision_id_for_navigation_id(),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .post_revisions()
        .retrieve_with_view_context(
            &PostEndpointType::Navigation,
            &navigation_id(),
            &revision_id_for_navigation_id(),
        )
        .await
        .assert_response();
}

fn navigation_id() -> PostId {
    PostId(TestCredentials::instance().navigation_id)
}

fn revision_id_for_navigation_id() -> PostRevisionId {
    PostRevisionId(TestCredentials::instance().revision_id_for_navigation_id)
}

#[template]
#[rstest]
#[case::default(AnyPostRevisionListParams::default())]
#[case::page(generate!(AnyPostRevisionListParams, (page, Some(1))))]
#[case::per_page(generate!(AnyPostRevisionListParams, (per_page, Some(3))))]
#[case::search(generate!(AnyPostRevisionListParams, (search, Some("foo".to_string()))))]
#[case::exclude(generate!(AnyPostRevisionListParams, (exclude, vec![PostRevisionId(1), PostRevisionId(2)])))]
#[case::include(generate!(AnyPostRevisionListParams, (include, vec![PostRevisionId(1)])))]
#[case::offset(generate!(AnyPostRevisionListParams, (offset, Some(5))))]
#[case::order(generate!(AnyPostRevisionListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(AnyPostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Slug))))]
fn list_cases(#[case] params: AnyPostRevisionListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_any_post_revision_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_any_post_revision_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_any_post_revision_field_with_view_context_test_cases!();

    #[apply(sparse_any_post_revision_field_with_edit_context_test_cases)]
    #[case(&[SparseAnyPostRevisionFieldWithEditContext::Id, SparseAnyPostRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparseAnyPostRevisionFieldWithEditContext],
        #[values(
            AnyPostRevisionListParams::default(),
            generate!(AnyPostRevisionListParams, (exclude, vec![PostRevisionId(2), PostRevisionId(3)])),
            generate!(AnyPostRevisionListParams, (search, Some("foo".to_string())))
        )]
        params: AnyPostRevisionListParams,
    ) {
        api_client()
            .post_revisions()
            .filter_list_with_edit_context(
                &PostEndpointType::Navigation,
                &navigation_id(),
                &params,
                fields,
            )
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|post| {
                post.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_any_post_revision_field_with_embed_context_test_cases)]
    #[case(&[SparseAnyPostRevisionFieldWithEmbedContext::Id, SparseAnyPostRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseAnyPostRevisionFieldWithEmbedContext],
        #[values(
            AnyPostRevisionListParams::default(),
            generate!(AnyPostRevisionListParams, (exclude, vec![PostRevisionId(2), PostRevisionId(3)])),
            generate!(AnyPostRevisionListParams, (search, Some("foo".to_string())))
        )]
        params: AnyPostRevisionListParams,
    ) {
        api_client()
            .post_revisions()
            .filter_list_with_embed_context(
                &PostEndpointType::Navigation,
                &navigation_id(),
                &params,
                fields,
            )
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|post| {
                post.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_any_post_revision_field_with_view_context_test_cases)]
    #[case(&[SparseAnyPostRevisionFieldWithViewContext::Id, SparseAnyPostRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseAnyPostRevisionFieldWithViewContext],
        #[values(
            AnyPostRevisionListParams::default(),
            generate!(AnyPostRevisionListParams, (exclude, vec![PostRevisionId(2), PostRevisionId(3)])),
            generate!(AnyPostRevisionListParams, (search, Some("foo".to_string())))
        )]
        params: AnyPostRevisionListParams,
    ) {
        api_client()
            .post_revisions()
            .filter_list_with_view_context(
                &PostEndpointType::Navigation,
                &navigation_id(),
                &params,
                fields,
            )
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|post| {
                post.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_any_post_revision_field_with_edit_context_test_cases)]
    #[case(&[SparseAnyPostRevisionFieldWithEditContext::Id, SparseAnyPostRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparseAnyPostRevisionFieldWithEditContext],
    ) {
        api_client()
            .post_revisions()
            .filter_retrieve_with_edit_context(
                &PostEndpointType::Navigation,
                &navigation_id(),
                &revision_id_for_navigation_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_any_post_revision_field_with_embed_context_test_cases)]
    #[case(&[SparseAnyPostRevisionFieldWithEmbedContext::Id, SparseAnyPostRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparseAnyPostRevisionFieldWithEmbedContext],
    ) {
        api_client()
            .post_revisions()
            .filter_retrieve_with_embed_context(
                &PostEndpointType::Navigation,
                &navigation_id(),
                &revision_id_for_navigation_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_any_post_revision_field_with_view_context_test_cases)]
    #[case(&[SparseAnyPostRevisionFieldWithViewContext::Id, SparseAnyPostRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparseAnyPostRevisionFieldWithViewContext],
    ) {
        api_client()
            .post_revisions()
            .filter_retrieve_with_view_context(
                &PostEndpointType::Navigation,
                &navigation_id(),
                &revision_id_for_navigation_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}
