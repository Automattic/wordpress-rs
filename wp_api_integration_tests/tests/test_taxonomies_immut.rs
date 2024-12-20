use rstest::rstest;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::post_types::PostType;
use wp_api::taxonomies::{
    SparseTaxonomyTypeDetailsFieldWithEditContext, SparseTaxonomyTypeDetailsFieldWithEmbedContext,
    SparseTaxonomyTypeDetailsFieldWithViewContext, TaxonomyListParams, TaxonomyType,
};
use wp_api_integration_tests::{api_client, AssertResponse};

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_taxonomies_with_edit_context(#[case] params: TaxonomyListParams) {
    let response = api_client()
        .taxonomies()
        .list_with_edit_context(&params)
        .await
        .assert_response()
        .data;
    assert_eq!(
        response
            .taxonomy_types
            .get(&TaxonomyType::Category)
            .expect("Our local WordPress test site has `category` taxonomy type")
            .name,
        "Categories"
    );
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_taxonomies_with_embed_context(#[case] params: TaxonomyListParams) {
    let response = api_client()
        .taxonomies()
        .list_with_embed_context(&params)
        .await
        .assert_response()
        .data;
    assert_eq!(
        response
            .taxonomy_types
            .get(&TaxonomyType::Category)
            .expect("Our local WordPress test site has `category` taxonomy type")
            .name,
        "Categories"
    );
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_taxonomies_with_view_context(#[case] params: TaxonomyListParams) {
    let response = api_client()
        .taxonomies()
        .list_with_view_context(&params)
        .await
        .assert_response()
        .data;
    assert_eq!(
        response
            .taxonomy_types
            .get(&TaxonomyType::Category)
            .expect("Our local WordPress test site has `category` taxonomy type")
            .name,
        "Categories"
    );
}

#[tokio::test]
#[apply(retrieve_cases)]
#[parallel]
async fn retrieve_taxonomies_with_edit_context(#[case] taxonomy_type: TaxonomyType) {
    api_client()
        .taxonomies()
        .retrieve_with_edit_context(&taxonomy_type)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(retrieve_cases)]
#[parallel]
async fn retrieve_taxonomies_with_embed_context(#[case] taxonomy_type: TaxonomyType) {
    api_client()
        .taxonomies()
        .retrieve_with_embed_context(&taxonomy_type)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(retrieve_cases)]
#[parallel]
async fn retrieve_taxonomies_with_view_context(#[case] taxonomy_type: TaxonomyType) {
    api_client()
        .taxonomies()
        .retrieve_with_view_context(&taxonomy_type)
        .await
        .assert_response();
}

#[template]
#[rstest]
#[case::default(TaxonomyListParams::default())]
#[case::post(TaxonomyListParams { post_type: Some(PostType::Post) })]
// TODO: These post types return:
// ```
// {"code":"rest_cannot_view","message":"Sorry, you are not allowed to manage terms in this taxonomy.","data":{"status":403}}
// ```
//#[case::page(TaxonomyListParams { post_type: Some(PostType::Page) })]
//#[case::attachment(TaxonomyListParams { post_type: Some(PostType::Attachment) })]
//#[case::nav_menu_item(TaxonomyListParams { post_type: Some(PostType::NavMenuItem) })]
//#[case::wp_block(TaxonomyListParams { post_type: Some(PostType::WpBlock) })]
//#[case::wp_template(TaxonomyListParams { post_type: Some(PostType::WpTemplate) })]
//#[case::wp_template_part(TaxonomyListParams { post_type: Some(PostType::WpTemplatePart) })]
//#[case::wp_navigation(TaxonomyListParams { post_type: Some(PostType::WpNavigation) })]
//#[case::wp_font_family(TaxonomyListParams { post_type: Some(PostType::WpFontFamily) })]
//#[case::wp_font_face(TaxonomyListParams { post_type: Some(PostType::WpFontFace) })]
pub fn list_cases(#[case] params: TaxonomyListParams) {}

#[template]
#[rstest]
#[case::category(TaxonomyType::Category)]
#[case::nav_menu(TaxonomyType::NavMenu)]
#[case::post_tag(TaxonomyType::PostTag)]
#[case::wp_pattern_category(TaxonomyType::WpPatternCategory)]
pub fn retrieve_cases(#[case] taxonomy_type: TaxonomyType) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_taxonomy_type_details_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_taxonomy_type_details_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_taxonomy_type_details_field_with_view_context_test_cases!();

    #[apply(sparse_taxonomy_type_details_field_with_edit_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_taxonomies_with_edit_context(
        #[values(
            TaxonomyType::Category,
            TaxonomyType::NavMenu,
            TaxonomyType::PostTag,
            TaxonomyType::WpPatternCategory
        )]
        taxonomy_type: TaxonomyType,
        #[case] fields: &[SparseTaxonomyTypeDetailsFieldWithEditContext],
    ) {
        let taxonomy = api_client()
            .taxonomies()
            .filter_retrieve_with_edit_context(&taxonomy_type, fields)
            .await
            .assert_response()
            .data;
        taxonomy.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_taxonomy_type_details_field_with_embed_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_taxonomies_with_embed_context(
        #[values(
            TaxonomyType::Category,
            TaxonomyType::NavMenu,
            TaxonomyType::PostTag,
            TaxonomyType::WpPatternCategory
        )]
        taxonomy_type: TaxonomyType,
        #[case] fields: &[SparseTaxonomyTypeDetailsFieldWithEmbedContext],
    ) {
        let taxonomy = api_client()
            .taxonomies()
            .filter_retrieve_with_embed_context(&taxonomy_type, fields)
            .await
            .assert_response()
            .data;
        taxonomy.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_taxonomy_type_details_field_with_view_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_taxonomies_with_view_context(
        #[values(
            TaxonomyType::Category,
            TaxonomyType::NavMenu,
            TaxonomyType::PostTag,
            TaxonomyType::WpPatternCategory
        )]
        taxonomy_type: TaxonomyType,
        #[case] fields: &[SparseTaxonomyTypeDetailsFieldWithViewContext],
    ) {
        let taxonomy = api_client()
            .taxonomies()
            .filter_retrieve_with_view_context(&taxonomy_type, fields)
            .await
            .assert_response()
            .data;
        taxonomy.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}
