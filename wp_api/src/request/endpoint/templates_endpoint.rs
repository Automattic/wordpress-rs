use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::SparseField;
use crate::templates::{
    SparseTemplateFieldWithEditContext, SparseTemplateFieldWithEmbedContext,
    SparseTemplateFieldWithViewContext, TemplateId,
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum TemplatesRequest {
    #[contextual_get(url = "/templates", params = &crate::templates::TemplateListParams, output = Vec<crate::templates::SparseTemplate>, filter_by = crate::templates::SparseTemplateField)]
    List,
    #[contextual_get(url = "/templates/<template_id>", output = crate::templates::SparseTemplate, filter_by = crate::templates::SparseTemplateField)]
    Retrieve,
    #[delete(url = "/templates/<template_id>", output = crate::templates::TemplateDeleteResponse)]
    Delete,
    #[delete(url = "/templates/<template_id>", output = crate::templates::TemplateWithEditContext)]
    Trash,
    #[post(url = "/templates/<template_id>", params = &crate::templates::TemplateUpdateParams, output = crate::templates::TemplateWithEditContext)]
    Update,
}

impl DerivedRequest for TemplatesRequest {
    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        match self {
            TemplatesRequest::Delete => vec![("force", true.to_string())],
            TemplatesRequest::Trash => vec![("force", false.to_string())],
            _ => vec![],
        }
    }

    fn namespace() -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

impl SparseField for SparseTemplateFieldWithEditContext {
    fn as_str(&self) -> &str {
        match self {
            Self::TemplateType => "type",
            Self::PostId => "wp_id",
            _ => self.as_field_name(),
        }
    }
}

impl SparseField for SparseTemplateFieldWithEmbedContext {
    fn as_str(&self) -> &str {
        match self {
            Self::TemplateType => "type",
            Self::PostId => "wp_id",
            _ => self.as_field_name(),
        }
    }
}

impl SparseField for SparseTemplateFieldWithViewContext {
    fn as_str(&self) -> &str {
        match self {
            Self::TemplateType => "type",
            Self::PostId => "wp_id",
            _ => self.as_field_name(),
        }
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
        templates::{TemplateArea, TemplateListParams},
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case(TemplateListParams::default(), "")]
    #[case(generate!(TemplateListParams, (post_id, Some(PostId(2)))), "wp_id=2")]
    #[case(generate!(TemplateListParams, (area, Some(TemplateArea::Header))), "area=header")]
    #[case(generate!(TemplateListParams, (area, Some(TemplateArea::Footer))), "area=footer")]
    #[case(generate!(TemplateListParams, (area, Some(TemplateArea::Uncategorized))), "area=uncategorized")]
    #[case(generate!(TemplateListParams, (post_type, Some(PostType::Post))), "post_type=post")]
    #[case(generate!(TemplateListParams, (post_type, Some(PostType::Page))), "post_type=page")]
    #[case(generate!(TemplateListParams, (post_type, Some(PostType::Attachment))), "post_type=attachment")]
    #[case(generate!(TemplateListParams, (post_type, Some(PostType::NavMenuItem))), "post_type=nav_menu_item")]
    #[case(generate!(TemplateListParams, (post_type, Some(PostType::WpBlock))), "post_type=wp_block")]
    #[case(generate!(TemplateListParams, (post_type, Some(PostType::WpTemplate))), "post_type=wp_template")]
    #[case(generate!(TemplateListParams, (post_type, Some(PostType::WpTemplatePart))), "post_type=wp_template_part")]
    #[case(generate!(TemplateListParams, (post_type, Some(PostType::WpNavigation))), "post_type=wp_navigation")]
    #[case(generate!(TemplateListParams, (post_type, Some(PostType::WpFontFamily))), "post_type=wp_font_family")]
    #[case(generate!(TemplateListParams, (post_type, Some(PostType::WpFontFace))), "post_type=wp_font_face")]
    #[case(
        template_list_params_with_all_fields(),
        EXPECTED_QUERY_PAIRS_FOR_TEMPLATE_LIST_PARAMS_WITH_ALL_FIELDS
    )]
    fn list_templates(
        endpoint: TemplatesRequestEndpoint,
        #[case] params: TemplateListParams,
        #[case] expected_additional_params: &str,
    ) {
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/templates?context={}", context)
            } else {
                format!(
                    "/templates?context={}&{}",
                    context, expected_additional_params
                )
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
    #[case(TemplateListParams::default(), &[], "/templates?context=view&_fields=")]
    #[case(generate!(TemplateListParams, (area, Some(TemplateArea::Footer))), &[SparseTemplateFieldWithViewContext::Author], "/templates?context=view&area=footer&_fields=author")]
    #[case(template_list_params_with_all_fields(), ALL_SPARSE_TEMPLATE_FIELDS_WITH_VIEW_CONTEXT, &format!("/templates?context=view&{}&{}", EXPECTED_QUERY_PAIRS_FOR_TEMPLATE_LIST_PARAMS_WITH_ALL_FIELDS, EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_TEMPLATE_FIELDS_WITH_VIEW_CONTEXT))]
    fn filter_list_template_with_view_context(
        endpoint: TemplatesRequestEndpoint,
        #[case] params: TemplateListParams,
        #[case] fields: &[SparseTemplateFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_view_context(&params, fields),
            expected_path,
        );
    }

    #[rstest]
    fn retrieve_template(endpoint: TemplatesRequestEndpoint) {
        let template_id = TemplateId("foo".to_string());
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&template_id),
            "/templates/foo?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&template_id),
            "/templates/foo?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&template_id),
            "/templates/foo?context=view",
        );
    }

    #[rstest]
    #[case(&[], "/templates/foo?context=view&_fields=")]
    #[case(&[SparseTemplateFieldWithViewContext::Slug], "/templates/foo?context=view&_fields=slug")]
    #[case(ALL_SPARSE_TEMPLATE_FIELDS_WITH_VIEW_CONTEXT, &format!("/templates/foo?context=view&{}", EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_TEMPLATE_FIELDS_WITH_VIEW_CONTEXT))]
    fn filter_retrieve_template_with_view_context(
        endpoint: TemplatesRequestEndpoint,
        #[case] fields: &[SparseTemplateFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(&TemplateId("foo".to_string()), fields),
            expected_path,
        );
    }

    #[rstest]
    fn delete_template(endpoint: TemplatesRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.delete(&TemplateId("foo".to_string())),
            "/templates/foo?force=true",
        );
    }

    #[rstest]
    fn trash_template(endpoint: TemplatesRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.trash(&TemplateId("foo".to_string())),
            "/templates/foo?force=false",
        );
    }

    #[rstest]
    fn update_template(endpoint: TemplatesRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.update(&TemplateId("foo".to_string())),
            "/templates/foo",
        );
    }

    const EXPECTED_QUERY_PAIRS_FOR_TEMPLATE_LIST_PARAMS_WITH_ALL_FIELDS: &str =
        "wp_id=2&area=header&post_type=page";
    fn template_list_params_with_all_fields() -> TemplateListParams {
        TemplateListParams {
            post_id: Some(PostId(2)),
            area: Some(TemplateArea::Header),
            post_type: Some(PostType::Page),
        }
    }

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_TEMPLATE_FIELDS_WITH_VIEW_CONTEXT: &str = "_fields=id%2Cslug%2Ctheme%2Ctype%2Csource%2Corigin%2Ccontent%2Ctitle%2Cdescription%2Cstatus%2Cwp_id%2Chas_theme_file%2Cauthor%2Cmodified%2Cis_custom%2Cauthor_text%2Coriginal_source";
    const ALL_SPARSE_TEMPLATE_FIELDS_WITH_VIEW_CONTEXT: &[SparseTemplateFieldWithViewContext; 17] =
        &[
            SparseTemplateFieldWithViewContext::Id,
            SparseTemplateFieldWithViewContext::Slug,
            SparseTemplateFieldWithViewContext::Theme,
            SparseTemplateFieldWithViewContext::TemplateType,
            SparseTemplateFieldWithViewContext::Source,
            SparseTemplateFieldWithViewContext::Origin,
            SparseTemplateFieldWithViewContext::Content,
            SparseTemplateFieldWithViewContext::Title,
            SparseTemplateFieldWithViewContext::Description,
            SparseTemplateFieldWithViewContext::Status,
            SparseTemplateFieldWithViewContext::PostId,
            SparseTemplateFieldWithViewContext::HasThemeFile,
            SparseTemplateFieldWithViewContext::Author,
            SparseTemplateFieldWithViewContext::Modified,
            SparseTemplateFieldWithViewContext::IsCustom,
            SparseTemplateFieldWithViewContext::AuthorText,
            SparseTemplateFieldWithViewContext::OriginalSource,
        ];

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> TemplatesRequestEndpoint {
        TemplatesRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
