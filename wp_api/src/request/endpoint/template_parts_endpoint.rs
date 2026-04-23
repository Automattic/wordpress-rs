use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::template_parts::TemplatePartId;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum TemplatePartsRequest {
    #[contextual_get(url = "/template-parts", params = &crate::template_parts::TemplatePartListParams, output = Vec<crate::template_parts::SparseTemplatePart>, filter_by = crate::template_parts::SparseTemplatePartField)]
    List,
    #[contextual_get(url = "/template-parts/<template_part_id>", output = crate::template_parts::SparseTemplatePart, filter_by = crate::template_parts::SparseTemplatePartField)]
    Retrieve,
}

impl DerivedRequest for TemplatePartsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        generate,
        post_types::PostType,
        posts::PostId,
        request::endpoint::{
            ApiUrlResolver,
            tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
        },
        template_parts::{
            SparseTemplatePartFieldWithViewContext, TemplatePartId, TemplatePartListParams,
        },
        templates::TemplateArea,
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case(TemplatePartListParams::default(), "")]
    #[case(generate!(TemplatePartListParams, (post_id, Some(PostId(2)))), "wp_id=2")]
    #[case(generate!(TemplatePartListParams, (area, Some(TemplateArea::Header))), "area=header")]
    #[case(generate!(TemplatePartListParams, (area, Some(TemplateArea::Footer))), "area=footer")]
    #[case(generate!(TemplatePartListParams, (area, Some(TemplateArea::Uncategorized))), "area=uncategorized")]
    #[case(generate!(TemplatePartListParams, (post_type, Some(PostType::Post))), "post_type=post")]
    #[case(generate!(TemplatePartListParams, (post_type, Some(PostType::Page))), "post_type=page")]
    #[case(generate!(TemplatePartListParams, (post_type, Some(PostType::Attachment))), "post_type=attachment")]
    #[case(generate!(TemplatePartListParams, (post_type, Some(PostType::NavMenuItem))), "post_type=nav_menu_item")]
    #[case(generate!(TemplatePartListParams, (post_type, Some(PostType::WpBlock))), "post_type=wp_block")]
    #[case(generate!(TemplatePartListParams, (post_type, Some(PostType::WpTemplate))), "post_type=wp_template")]
    #[case(generate!(TemplatePartListParams, (post_type, Some(PostType::WpTemplatePart))), "post_type=wp_template_part")]
    #[case(generate!(TemplatePartListParams, (post_type, Some(PostType::WpNavigation))), "post_type=wp_navigation")]
    #[case(generate!(TemplatePartListParams, (post_type, Some(PostType::WpFontFamily))), "post_type=wp_font_family")]
    #[case(generate!(TemplatePartListParams, (post_type, Some(PostType::WpFontFace))), "post_type=wp_font_face")]
    #[case(
        template_part_list_params_with_all_fields(),
        EXPECTED_QUERY_PAIRS_FOR_TEMPLATE_PART_LIST_PARAMS_WITH_ALL_FIELDS
    )]
    fn list_template_parts(
        endpoint: TemplatePartsRequestEndpoint,
        #[case] params: TemplatePartListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/template-parts?context={context}")
            } else {
                format!("/template-parts?context={context}&{expected_additional_params}")
            }
        };
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&params),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(&params),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(&params),
            &expected_path("view"),
        );
    }

    #[rstest]
    #[case(TemplatePartListParams::default(), &[], "/template-parts?context=view&_fields=")]
    #[case(generate!(TemplatePartListParams, (area, Some(TemplateArea::Footer))), &[SparseTemplatePartFieldWithViewContext::Author], "/template-parts?context=view&area=footer&_fields=author")]
    #[case(template_part_list_params_with_all_fields(), ALL_SPARSE_TEMPLATE_PART_FIELDS_WITH_VIEW_CONTEXT, &format!("/template-parts?context=view&{EXPECTED_QUERY_PAIRS_FOR_TEMPLATE_PART_LIST_PARAMS_WITH_ALL_FIELDS}&{EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_TEMPLATE_PART_FIELDS_WITH_VIEW_CONTEXT}"))]
    fn filter_list_template_part_with_view_context(
        endpoint: TemplatePartsRequestEndpoint,
        #[case] params: TemplatePartListParams,
        #[case] fields: &[SparseTemplatePartFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_view_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    fn retrieve_template_part(endpoint: TemplatePartsRequestEndpoint) {
        let template_part_id = TemplatePartId("foo".to_string());
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&template_part_id),
            "/template-parts/foo?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&template_part_id),
            "/template-parts/foo?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&template_part_id),
            "/template-parts/foo?context=view",
        );
    }

    #[rstest]
    #[case(&[], "/template-parts/foo?context=view&_fields=")]
    #[case(&[SparseTemplatePartFieldWithViewContext::Slug], "/template-parts/foo?context=view&_fields=slug")]
    #[case(ALL_SPARSE_TEMPLATE_PART_FIELDS_WITH_VIEW_CONTEXT, &format!("/template-parts/foo?context=view&{EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_TEMPLATE_PART_FIELDS_WITH_VIEW_CONTEXT}"))]
    fn filter_retrieve_template_part_with_view_context(
        endpoint: TemplatePartsRequestEndpoint,
        #[case] fields: &[SparseTemplatePartFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(&TemplatePartId("foo".to_string()), fields),
            expected_path,
        );
    }

    const EXPECTED_QUERY_PAIRS_FOR_TEMPLATE_PART_LIST_PARAMS_WITH_ALL_FIELDS: &str =
        "wp_id=2&area=header&post_type=page";
    fn template_part_list_params_with_all_fields() -> TemplatePartListParams {
        TemplatePartListParams {
            post_id: Some(PostId(2)),
            area: Some(TemplateArea::Header),
            post_type: Some(PostType::Page),
        }
    }

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_TEMPLATE_PART_FIELDS_WITH_VIEW_CONTEXT: &str = "_fields=id%2Cslug%2Ctheme%2Ctype%2Csource%2Corigin%2Ccontent%2Ctitle%2Cdescription%2Cstatus%2Cwp_id%2Chas_theme_file%2Cauthor%2Cmodified%2Carea";
    const ALL_SPARSE_TEMPLATE_PART_FIELDS_WITH_VIEW_CONTEXT: &[SparseTemplatePartFieldWithViewContext;
         15] = &[
        SparseTemplatePartFieldWithViewContext::Id,
        SparseTemplatePartFieldWithViewContext::Slug,
        SparseTemplatePartFieldWithViewContext::Theme,
        SparseTemplatePartFieldWithViewContext::TemplateType,
        SparseTemplatePartFieldWithViewContext::Source,
        SparseTemplatePartFieldWithViewContext::Origin,
        SparseTemplatePartFieldWithViewContext::Content,
        SparseTemplatePartFieldWithViewContext::Title,
        SparseTemplatePartFieldWithViewContext::Description,
        SparseTemplatePartFieldWithViewContext::Status,
        SparseTemplatePartFieldWithViewContext::PostId,
        SparseTemplatePartFieldWithViewContext::HasThemeFile,
        SparseTemplatePartFieldWithViewContext::Author,
        SparseTemplatePartFieldWithViewContext::Modified,
        SparseTemplatePartFieldWithViewContext::Area,
    ];

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> TemplatePartsRequestEndpoint {
        TemplatePartsRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
