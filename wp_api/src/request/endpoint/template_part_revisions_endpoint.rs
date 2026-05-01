use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::template_parts::TemplatePartId;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum TemplatePartRevisionsRequest {
    #[contextual_paged(url = "/template-parts/<template_part_id>/revisions", params = &crate::template_part_revisions::TemplatePartRevisionListParams, output = Vec<crate::template_part_revisions::SparseTemplatePartRevision>, filter_by = crate::template_part_revisions::SparseTemplatePartRevisionField)]
    List,
}

impl DerivedRequest for TemplatePartRevisionsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        WpApiParamOrder, generate,
        request::endpoint::{
            ApiUrlResolver,
            tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
        },
        template_part_revisions::{
            SparseTemplatePartRevisionFieldWithViewContext, TemplatePartRevisionId,
            TemplatePartRevisionListParams, WpApiParamTemplatePartRevisionsOrderBy,
        },
        template_parts::TemplatePartId,
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case(TemplatePartRevisionListParams::default(), "")]
    #[case(generate!(TemplatePartRevisionListParams, (page, Some(1))), "page=1")]
    #[case(generate!(TemplatePartRevisionListParams, (per_page, Some(3))), "per_page=3")]
    #[case(generate!(TemplatePartRevisionListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(TemplatePartRevisionListParams, (exclude, vec![TemplatePartRevisionId(1), TemplatePartRevisionId(2)])), "exclude=1%2C2")]
    #[case(generate!(TemplatePartRevisionListParams, (include, vec![TemplatePartRevisionId(1)])), "include=1")]
    #[case(generate!(TemplatePartRevisionListParams, (offset, Some(5))), "offset=5")]
    #[case(generate!(TemplatePartRevisionListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(TemplatePartRevisionListParams, (orderby, Some(WpApiParamTemplatePartRevisionsOrderBy::Slug))), "orderby=slug")]
    fn list_template_part_revisions(
        endpoint: TemplatePartRevisionsRequestEndpoint,
        #[case] params: TemplatePartRevisionListParams,
        #[case] expected_additional_params: &str,
    ) {
        let template_part_id = TemplatePartId("foo".to_string());
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/template-parts/foo/revisions?context={context}")
            } else {
                format!(
                    "/template-parts/foo/revisions?context={context}&{expected_additional_params}"
                )
            }
        };
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&template_part_id, &params),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(&template_part_id, &params),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(&template_part_id, &params),
            &expected_path("view"),
        );
    }

    #[rstest]
    #[case(TemplatePartRevisionListParams::default(), &[], "/template-parts/foo/revisions?context=view&_fields=")]
    #[case(generate!(TemplatePartRevisionListParams, (page, Some(1))), &[SparseTemplatePartRevisionFieldWithViewContext::Author], "/template-parts/foo/revisions?context=view&page=1&_fields=author")]
    fn filter_list_template_part_revisions_with_view_context(
        endpoint: TemplatePartRevisionsRequestEndpoint,
        #[case] params: TemplatePartRevisionListParams,
        #[case] fields: &[SparseTemplatePartRevisionFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_view_context(
                &TemplatePartId("foo".to_string()),
                &params,
                fields,
            ),
            expected_path,
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> TemplatePartRevisionsRequestEndpoint {
        TemplatePartRevisionsRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
