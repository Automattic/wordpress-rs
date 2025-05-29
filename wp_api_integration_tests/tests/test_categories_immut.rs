use wp_api::categories::{
    CategoryListParams, SparseCategoryFieldWithEditContext, SparseCategoryFieldWithEmbedContext,
    SparseCategoryFieldWithViewContext, WpApiParamCategoriesOrderBy,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: CategoryListParams) {
    api_client()
        .categories()
        .list_with_edit_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: CategoryListParams) {
    api_client()
        .categories()
        .list_with_embed_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: CategoryListParams) {
    api_client()
        .categories()
        .list_with_view_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .categories()
        .retrieve_with_edit_context(&CATEGORY_ID_59)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .categories()
        .retrieve_with_embed_context(&CATEGORY_ID_59)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .categories()
        .retrieve_with_view_context(&CATEGORY_ID_59)
        .await
        .assert_response();
}

#[template]
#[rstest]
#[case::default(CategoryListParams::default())]
#[case::page(generate!(CategoryListParams, (page, Some(1))))]
#[case::per_page(generate!(CategoryListParams, (per_page, Some(3))))]
#[case::search(generate!(CategoryListParams, (search, Some("foo".to_string()))))]
#[case::exclude(generate!(CategoryListParams, (exclude, vec![CATEGORY_ID_59])))]
#[case::include(generate!(CategoryListParams, (include, vec![CATEGORY_ID_59])))]
#[case::offset(generate!(CategoryListParams, (offset, Some(2))))]
#[case::order(generate!(CategoryListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(CategoryListParams, (orderby, Some(WpApiParamCategoriesOrderBy::Id))))]
#[case::hide_empty_false(generate!(CategoryListParams, (hide_empty, Some(false))))]
#[case::hide_empty_true(generate!(CategoryListParams, (hide_empty, Some(true))))]
#[case::post(generate!(CategoryListParams, (parent, Some(CATEGORY_ID_59))))]
#[case::post(generate!(CategoryListParams, (post, Some(FIRST_POST_ID))))]
#[case::slug(generate!(CategoryListParams, (slug, vec!["foo".to_string(), "bar".to_string()])))]
pub fn list_cases(#[case] params: CategoryListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_category_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_category_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_category_field_with_view_context_test_cases!();

    #[apply(sparse_category_field_with_edit_context_test_cases)]
    #[case(&[SparseCategoryFieldWithEditContext::Name, SparseCategoryFieldWithEditContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparseCategoryFieldWithEditContext],
        #[values(
            CategoryListParams::default(),
            generate!(CategoryListParams, (orderby, Some(WpApiParamCategoriesOrderBy::Id))),
            generate!(CategoryListParams, (search, Some("foo".to_string())))
        )]
        params: CategoryListParams,
    ) {
        api_client()
            .categories()
            .filter_list_with_edit_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|category| {
                category.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_category_field_with_edit_context_test_cases)]
    #[case(&[SparseCategoryFieldWithEditContext::Name, SparseCategoryFieldWithEditContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparseCategoryFieldWithEditContext],
    ) {
        let category = api_client()
            .categories()
            .filter_retrieve_with_edit_context(&CATEGORY_ID_59, fields)
            .await
            .assert_response()
            .data;
        category.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_category_field_with_embed_context_test_cases)]
    #[case(&[SparseCategoryFieldWithEmbedContext::Name, SparseCategoryFieldWithEmbedContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseCategoryFieldWithEmbedContext],
        #[values(
            CategoryListParams::default(),
            generate!(CategoryListParams, (orderby, Some(WpApiParamCategoriesOrderBy::Id))),
            generate!(CategoryListParams, (search, Some("foo".to_string())))
        )]
        params: CategoryListParams,
    ) {
        api_client()
            .categories()
            .filter_list_with_embed_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|category| {
                category.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_category_field_with_embed_context_test_cases)]
    #[case(&[SparseCategoryFieldWithEmbedContext::Name, SparseCategoryFieldWithEmbedContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparseCategoryFieldWithEmbedContext],
    ) {
        let category = api_client()
            .categories()
            .filter_retrieve_with_embed_context(&CATEGORY_ID_59, fields)
            .await
            .assert_response()
            .data;
        category.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_category_field_with_view_context_test_cases)]
    #[case(&[SparseCategoryFieldWithViewContext::Name, SparseCategoryFieldWithViewContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseCategoryFieldWithViewContext],
        #[values(
            CategoryListParams::default(),
            generate!(CategoryListParams, (orderby, Some(WpApiParamCategoriesOrderBy::Id))),
            generate!(CategoryListParams, (search, Some("foo".to_string())))
        )]
        params: CategoryListParams,
    ) {
        api_client()
            .categories()
            .filter_list_with_view_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|category| {
                category.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_category_field_with_view_context_test_cases)]
    #[case(&[SparseCategoryFieldWithViewContext::Name, SparseCategoryFieldWithViewContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparseCategoryFieldWithViewContext],
    ) {
        let category = api_client()
            .categories()
            .filter_retrieve_with_view_context(&CATEGORY_ID_59, fields)
            .await
            .assert_response()
            .data;
        category.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }
}
