use wp_api::request::endpoint::terms_endpoint::TermEndpointType;
use wp_api::terms::{
    SparseAnyTermFieldWithEditContext, SparseAnyTermFieldWithEmbedContext,
    SparseAnyTermFieldWithViewContext, TermListParams, WpApiParamTermsOrderBy,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: TermListParams) {
    api_client()
        .terms()
        .list_with_edit_context(&TermEndpointType::Categories, &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: TermListParams) {
    api_client()
        .terms()
        .list_with_embed_context(&TermEndpointType::Categories, &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: TermListParams) {
    api_client()
        .terms()
        .list_with_view_context(&TermEndpointType::Categories, &params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .terms()
        .retrieve_with_edit_context(&TermEndpointType::Categories, &CATEGORY_ID_59)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .terms()
        .retrieve_with_embed_context(&TermEndpointType::Categories, &CATEGORY_ID_59)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .terms()
        .retrieve_with_view_context(&TermEndpointType::Categories, &CATEGORY_ID_59)
        .await
        .assert_response();
}

#[template]
#[rstest]
#[case::default(TermListParams::default())]
#[case::page(generate!(TermListParams, (page, Some(1))))]
#[case::per_page(generate!(TermListParams, (per_page, Some(3))))]
#[case::search(generate!(TermListParams, (search, Some("foo".to_string()))))]
#[case::exclude(generate!(TermListParams, (exclude, vec![CATEGORY_ID_59])))]
#[case::include(generate!(TermListParams, (include, vec![CATEGORY_ID_59])))]
#[case::offset(generate!(TermListParams, (offset, Some(2))))]
#[case::order(generate!(TermListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::Id))))]
#[case::hide_empty_false(generate!(TermListParams, (hide_empty, Some(false))))]
#[case::hide_empty_true(generate!(TermListParams, (hide_empty, Some(true))))]
#[case::post(generate!(TermListParams, (parent, Some(CATEGORY_ID_59))))]
#[case::post(generate!(TermListParams, (post, Some(FIRST_POST_ID))))]
#[case::slug(generate!(TermListParams, (slug, vec!["foo".to_string(), "bar".to_string()])))]
pub fn list_cases(#[case] params: TermListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_any_term_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_any_term_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_any_term_field_with_view_context_test_cases!();

    #[apply(sparse_any_term_field_with_edit_context_test_cases)]
    #[case(&[SparseAnyTermFieldWithEditContext::Name, SparseAnyTermFieldWithEditContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparseAnyTermFieldWithEditContext],
        #[values(
            TermListParams::default(),
            generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::Id))),
            generate!(TermListParams, (search, Some("foo".to_string())))
        )]
        params: TermListParams,
    ) {
        api_client()
            .terms()
            .filter_list_with_edit_context(&TermEndpointType::Categories, &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|category| {
                category.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_any_term_field_with_edit_context_test_cases)]
    #[case(&[SparseAnyTermFieldWithEditContext::Name, SparseAnyTermFieldWithEditContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparseAnyTermFieldWithEditContext],
    ) {
        let category = api_client()
            .terms()
            .filter_retrieve_with_edit_context(
                &TermEndpointType::Categories,
                &CATEGORY_ID_59,
                fields,
            )
            .await
            .assert_response()
            .data;
        category.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_any_term_field_with_embed_context_test_cases)]
    #[case(&[SparseAnyTermFieldWithEmbedContext::Name, SparseAnyTermFieldWithEmbedContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseAnyTermFieldWithEmbedContext],
        #[values(
            TermListParams::default(),
            generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::Id))),
            generate!(TermListParams, (search, Some("foo".to_string())))
        )]
        params: TermListParams,
    ) {
        api_client()
            .terms()
            .filter_list_with_embed_context(&TermEndpointType::Categories, &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|category| {
                category.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_any_term_field_with_embed_context_test_cases)]
    #[case(&[SparseAnyTermFieldWithEmbedContext::Name, SparseAnyTermFieldWithEmbedContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparseAnyTermFieldWithEmbedContext],
    ) {
        let category = api_client()
            .terms()
            .filter_retrieve_with_embed_context(
                &TermEndpointType::Categories,
                &CATEGORY_ID_59,
                fields,
            )
            .await
            .assert_response()
            .data;
        category.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_any_term_field_with_view_context_test_cases)]
    #[case(&[SparseAnyTermFieldWithViewContext::Name, SparseAnyTermFieldWithViewContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseAnyTermFieldWithViewContext],
        #[values(
            TermListParams::default(),
            generate!(TermListParams, (orderby, Some(WpApiParamTermsOrderBy::Id))),
            generate!(TermListParams, (search, Some("foo".to_string())))
        )]
        params: TermListParams,
    ) {
        api_client()
            .terms()
            .filter_list_with_view_context(&TermEndpointType::Categories, &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|category| {
                category.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_any_term_field_with_view_context_test_cases)]
    #[case(&[SparseAnyTermFieldWithViewContext::Name, SparseAnyTermFieldWithViewContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparseAnyTermFieldWithViewContext],
    ) {
        let category = api_client()
            .terms()
            .filter_retrieve_with_view_context(
                &TermEndpointType::Categories,
                &CATEGORY_ID_59,
                fields,
            )
            .await
            .assert_response()
            .data;
        category.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }
}
