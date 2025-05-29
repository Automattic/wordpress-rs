use wp_api::{
    post_types::PostType,
    templates::{
        SparseTemplateFieldWithEditContext, SparseTemplateFieldWithEmbedContext,
        SparseTemplateFieldWithViewContext, TemplateArea, TemplateId, TemplateListParams,
    },
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: TemplateListParams) {
    api_client()
        .templates()
        .list_with_edit_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: TemplateListParams) {
    api_client()
        .templates()
        .list_with_embed_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: TemplateListParams) {
    api_client()
        .templates()
        .list_with_view_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    let template_id = template_id_for_retrieve_tests();
    let template = api_client()
        .templates()
        .retrieve_with_edit_context(&TemplateId(TEMPLATE_TWENTY_TWENTY_FOUR_SINGLE.to_string()))
        .await
        .assert_response()
        .data;
    assert_eq!(template_id, template.id);
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    let template_id = template_id_for_retrieve_tests();
    let template = api_client()
        .templates()
        .retrieve_with_embed_context(&TemplateId(TEMPLATE_TWENTY_TWENTY_FOUR_SINGLE.to_string()))
        .await
        .assert_response()
        .data;
    assert_eq!(template_id, template.id);
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    let template_id = template_id_for_retrieve_tests();
    let template = api_client()
        .templates()
        .retrieve_with_view_context(&template_id)
        .await
        .assert_response()
        .data;
    assert_eq!(template_id, template.id);
}

fn template_id_for_retrieve_tests() -> TemplateId {
    TemplateId(TEMPLATE_TWENTY_TWENTY_FOUR_SINGLE.to_string())
}

#[template]
#[rstest]
#[case::default(TemplateListParams::default())]
#[case::post_id_first_post(generate!(TemplateListParams, (post_id, Some(FIRST_POST_ID))))]
#[case::post_id_555(generate!(TemplateListParams, (post_id, Some(POST_ID_555))))]
#[case::post_id_draft(generate!(TemplateListParams, (post_id, Some(POST_ID_DRAFT))))]
#[case::area_header(generate!(TemplateListParams, (area, Some(TemplateArea::Header))))]
#[case::area_footer(generate!(TemplateListParams, (area, Some(TemplateArea::Footer))))]
#[case::area_uncategorized(generate!(TemplateListParams, (area, Some(TemplateArea::Uncategorized))))]
#[case::post_type_post(generate!(TemplateListParams, (post_type, Some(PostType::Post))))]
#[case::post_type_page(generate!(TemplateListParams, (post_type, Some(PostType::Page))))]
#[case::post_type_attachment(generate!(TemplateListParams, (post_type, Some(PostType::Attachment))))]
#[case::post_type_nav_menu_item(generate!(TemplateListParams, (post_type, Some(PostType::NavMenuItem))))]
#[case::post_type_wp_block(generate!(TemplateListParams, (post_type, Some(PostType::WpBlock))))]
#[case::post_type_wp_template(generate!(TemplateListParams, (post_type, Some(PostType::WpTemplate))))]
#[case::post_type_wp_template_part(generate!(TemplateListParams, (post_type, Some(PostType::WpTemplatePart))))]
#[case::post_type_wp_navigation(generate!(TemplateListParams, (post_type, Some(PostType::WpNavigation))))]
#[case::post_type_wp_font_family(generate!(TemplateListParams, (post_type, Some(PostType::WpFontFamily))))]
#[case::post_type_wp_font_face(generate!(TemplateListParams, (post_type, Some(PostType::WpFontFace))))]
pub fn list_cases(#[case] params: TemplateListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_template_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_template_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_template_field_with_view_context_test_cases!();

    #[apply(sparse_template_field_with_edit_context_test_cases)]
    #[case(&[SparseTemplateFieldWithEditContext::Slug, SparseTemplateFieldWithEditContext::Theme])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparseTemplateFieldWithEditContext],
        #[values(
            TemplateListParams::default(),
            generate!(TemplateListParams, (post_id, Some(FIRST_POST_ID))),
            generate!(TemplateListParams, (area, Some(TemplateArea::Header))),
            generate!(TemplateListParams, (post_type, Some(PostType::Post))),
            generate!(TemplateListParams, (post_type, Some(PostType::Page)))
        )]
        params: TemplateListParams,
    ) {
        api_client()
            .templates()
            .filter_list_with_edit_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|template| {
                template.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_template_field_with_edit_context_test_cases)]
    #[case(&[SparseTemplateFieldWithEditContext::Slug, SparseTemplateFieldWithEditContext::Theme])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparseTemplateFieldWithEditContext],
    ) {
        let template = api_client()
            .templates()
            .filter_retrieve_with_edit_context(&template_id_for_retrieve_tests(), fields)
            .await
            .assert_response()
            .data;
        template.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_template_field_with_embed_context_test_cases)]
    #[case(&[SparseTemplateFieldWithEmbedContext::Slug, SparseTemplateFieldWithEmbedContext::Theme])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseTemplateFieldWithEmbedContext],
        #[values(
            TemplateListParams::default(),
            generate!(TemplateListParams, (post_id, Some(FIRST_POST_ID))),
            generate!(TemplateListParams, (area, Some(TemplateArea::Header))),
            generate!(TemplateListParams, (post_type, Some(PostType::Post))),
            generate!(TemplateListParams, (post_type, Some(PostType::Page)))
        )]
        params: TemplateListParams,
    ) {
        api_client()
            .templates()
            .filter_list_with_embed_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|template| {
                template.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_template_field_with_embed_context_test_cases)]
    #[case(&[SparseTemplateFieldWithEmbedContext::Slug, SparseTemplateFieldWithEmbedContext::Theme])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparseTemplateFieldWithEmbedContext],
    ) {
        let template = api_client()
            .templates()
            .filter_retrieve_with_embed_context(&template_id_for_retrieve_tests(), fields)
            .await
            .assert_response()
            .data;
        template.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_template_field_with_view_context_test_cases)]
    #[case(&[SparseTemplateFieldWithViewContext::Slug, SparseTemplateFieldWithViewContext::Theme])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseTemplateFieldWithViewContext],
        #[values(
            TemplateListParams::default(),
            generate!(TemplateListParams, (post_id, Some(FIRST_POST_ID))),
            generate!(TemplateListParams, (area, Some(TemplateArea::Header))),
            generate!(TemplateListParams, (post_type, Some(PostType::Post))),
            generate!(TemplateListParams, (post_type, Some(PostType::Page)))
        )]
        params: TemplateListParams,
    ) {
        api_client()
            .templates()
            .filter_list_with_view_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|template| {
                template.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_template_field_with_view_context_test_cases)]
    #[case(&[SparseTemplateFieldWithViewContext::Slug, SparseTemplateFieldWithViewContext::Theme])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparseTemplateFieldWithViewContext],
    ) {
        let template = api_client()
            .templates()
            .filter_retrieve_with_view_context(&template_id_for_retrieve_tests(), fields)
            .await
            .assert_response()
            .data;
        template.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }
}
