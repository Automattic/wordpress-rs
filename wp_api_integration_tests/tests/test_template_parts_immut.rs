use wp_api::{
    post_types::PostType,
    template_parts::{
        SparseTemplatePartFieldWithEditContext, SparseTemplatePartFieldWithEmbedContext,
        SparseTemplatePartFieldWithViewContext, TemplatePartId, TemplatePartListParams,
    },
    templates::TemplateArea,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: TemplatePartListParams) {
    api_client()
        .template_parts()
        .list_with_edit_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: TemplatePartListParams) {
    api_client()
        .template_parts()
        .list_with_embed_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: TemplatePartListParams) {
    api_client()
        .template_parts()
        .list_with_view_context(&params)
        .await
        .assert_response();
}

#[template]
#[rstest]
#[case::default(TemplatePartListParams::default())]
#[case::post_id_first_post(generate!(TemplatePartListParams, (post_id, Some(FIRST_POST_ID))))]
#[case::post_id_555(generate!(TemplatePartListParams, (post_id, Some(POST_ID_555))))]
#[case::post_id_draft(generate!(TemplatePartListParams, (post_id, Some(POST_ID_DRAFT))))]
#[case::area_header(generate!(TemplatePartListParams, (area, Some(TemplateArea::Header))))]
#[case::area_footer(generate!(TemplatePartListParams, (area, Some(TemplateArea::Footer))))]
#[case::area_uncategorized(generate!(TemplatePartListParams, (area, Some(TemplateArea::Uncategorized))))]
#[case::post_type_post(generate!(TemplatePartListParams, (post_type, Some(PostType::Post))))]
#[case::post_type_page(generate!(TemplatePartListParams, (post_type, Some(PostType::Page))))]
#[case::post_type_attachment(generate!(TemplatePartListParams, (post_type, Some(PostType::Attachment))))]
#[case::post_type_nav_menu_item(generate!(TemplatePartListParams, (post_type, Some(PostType::NavMenuItem))))]
#[case::post_type_wp_block(generate!(TemplatePartListParams, (post_type, Some(PostType::WpBlock))))]
#[case::post_type_wp_template(generate!(TemplatePartListParams, (post_type, Some(PostType::WpTemplate))))]
#[case::post_type_wp_template_part(generate!(TemplatePartListParams, (post_type, Some(PostType::WpTemplatePart))))]
#[case::post_type_wp_navigation(generate!(TemplatePartListParams, (post_type, Some(PostType::WpNavigation))))]
#[case::post_type_wp_font_family(generate!(TemplatePartListParams, (post_type, Some(PostType::WpFontFamily))))]
#[case::post_type_wp_font_face(generate!(TemplatePartListParams, (post_type, Some(PostType::WpFontFace))))]
pub fn list_cases(#[case] params: TemplatePartListParams) {}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    let template_part_id = template_part_id_for_retrieve_tests();
    let template_part = api_client()
        .template_parts()
        .retrieve_with_edit_context(&TemplatePartId(
            TEMPLATE_PART_TWENTY_TWENTY_FOUR_HEADER.to_string(),
        ))
        .await
        .assert_response()
        .data;
    assert_eq!(template_part_id, template_part.id);
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    let template_part_id = template_part_id_for_retrieve_tests();
    let template_part = api_client()
        .template_parts()
        .retrieve_with_embed_context(&TemplatePartId(
            TEMPLATE_PART_TWENTY_TWENTY_FOUR_HEADER.to_string(),
        ))
        .await
        .assert_response()
        .data;
    assert_eq!(template_part_id, template_part.id);
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    let template_part_id = template_part_id_for_retrieve_tests();
    let template_part = api_client()
        .template_parts()
        .retrieve_with_view_context(&template_part_id)
        .await
        .assert_response()
        .data;
    assert_eq!(template_part_id, template_part.id);
}

fn template_part_id_for_retrieve_tests() -> TemplatePartId {
    TemplatePartId(TEMPLATE_PART_TWENTY_TWENTY_FOUR_HEADER.to_string())
}

mod filter {
    use super::*;

    wp_api::generate_sparse_template_part_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_template_part_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_template_part_field_with_view_context_test_cases!();

    #[apply(sparse_template_part_field_with_edit_context_test_cases)]
    #[case(&[SparseTemplatePartFieldWithEditContext::Slug, SparseTemplatePartFieldWithEditContext::Theme])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparseTemplatePartFieldWithEditContext],
    ) {
        let template_part = api_client()
            .template_parts()
            .filter_retrieve_with_edit_context(&template_part_id_for_retrieve_tests(), fields)
            .await
            .assert_response()
            .data;
        template_part.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_template_part_field_with_edit_context_test_cases)]
    #[case(&[SparseTemplatePartFieldWithEditContext::Slug, SparseTemplatePartFieldWithEditContext::Theme])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparseTemplatePartFieldWithEditContext],
        #[values(
            TemplatePartListParams::default(),
            generate!(TemplatePartListParams, (post_id, Some(FIRST_POST_ID))),
            generate!(TemplatePartListParams, (area, Some(TemplateArea::Header))),
            generate!(TemplatePartListParams, (post_type, Some(PostType::Post))),
            generate!(TemplatePartListParams, (post_type, Some(PostType::Page)))
        )]
        params: TemplatePartListParams,
    ) {
        api_client()
            .template_parts()
            .filter_list_with_edit_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|template_part| {
                template_part.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_template_part_field_with_embed_context_test_cases)]
    #[case(&[SparseTemplatePartFieldWithEmbedContext::Slug, SparseTemplatePartFieldWithEmbedContext::Theme])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparseTemplatePartFieldWithEmbedContext],
    ) {
        let template_part = api_client()
            .template_parts()
            .filter_retrieve_with_embed_context(&template_part_id_for_retrieve_tests(), fields)
            .await
            .assert_response()
            .data;
        template_part.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_template_part_field_with_embed_context_test_cases)]
    #[case(&[SparseTemplatePartFieldWithEmbedContext::Slug, SparseTemplatePartFieldWithEmbedContext::Theme])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseTemplatePartFieldWithEmbedContext],
        #[values(
            TemplatePartListParams::default(),
            generate!(TemplatePartListParams, (post_id, Some(FIRST_POST_ID))),
            generate!(TemplatePartListParams, (area, Some(TemplateArea::Header))),
            generate!(TemplatePartListParams, (post_type, Some(PostType::Post))),
            generate!(TemplatePartListParams, (post_type, Some(PostType::Page)))
        )]
        params: TemplatePartListParams,
    ) {
        api_client()
            .template_parts()
            .filter_list_with_embed_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|template_part| {
                template_part.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_template_part_field_with_view_context_test_cases)]
    #[case(&[SparseTemplatePartFieldWithViewContext::Slug, SparseTemplatePartFieldWithViewContext::Theme])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparseTemplatePartFieldWithViewContext],
    ) {
        let template_part = api_client()
            .template_parts()
            .filter_retrieve_with_view_context(&template_part_id_for_retrieve_tests(), fields)
            .await
            .assert_response()
            .data;
        template_part.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_template_part_field_with_view_context_test_cases)]
    #[case(&[SparseTemplatePartFieldWithViewContext::Slug, SparseTemplatePartFieldWithViewContext::Theme])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseTemplatePartFieldWithViewContext],
        #[values(
            TemplatePartListParams::default(),
            generate!(TemplatePartListParams, (post_id, Some(FIRST_POST_ID))),
            generate!(TemplatePartListParams, (area, Some(TemplateArea::Header))),
            generate!(TemplatePartListParams, (post_type, Some(PostType::Post))),
            generate!(TemplatePartListParams, (post_type, Some(PostType::Page)))
        )]
        params: TemplatePartListParams,
    ) {
        api_client()
            .template_parts()
            .filter_list_with_view_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|template_part| {
                template_part.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }
}
