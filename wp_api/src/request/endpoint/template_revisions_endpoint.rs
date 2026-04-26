use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{template_revisions::TemplateRevisionId, templates::TemplateId};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum TemplateRevisionsRequest {
    #[contextual_paged(url = "/templates/<template_id>/revisions", params = &crate::template_revisions::TemplateRevisionListParams, output = Vec<crate::template_revisions::SparseTemplateRevision>, filter_by = crate::template_revisions::SparseTemplateRevisionField)]
    List,
    #[contextual_get(url = "/templates/<template_id>/revisions/<template_revision_id>", output = crate::template_revisions::SparseTemplateRevision, filter_by = crate::template_revisions::SparseTemplateRevisionField)]
    Retrieve,
}

impl DerivedRequest for TemplateRevisionsRequest {
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
        template_revisions::{
            SparseTemplateRevisionFieldWithViewContext, TemplateRevisionId,
            TemplateRevisionListParams, WpApiParamTemplateRevisionsOrderBy,
        },
        templates::TemplateId,
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case(TemplateRevisionListParams::default(), "")]
    #[case(generate!(TemplateRevisionListParams, (page, Some(1))), "page=1")]
    #[case(generate!(TemplateRevisionListParams, (per_page, Some(3))), "per_page=3")]
    #[case(generate!(TemplateRevisionListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(TemplateRevisionListParams, (exclude, vec![TemplateRevisionId(1), TemplateRevisionId(2)])), "exclude=1%2C2")]
    #[case(generate!(TemplateRevisionListParams, (include, vec![TemplateRevisionId(1)])), "include=1")]
    #[case(generate!(TemplateRevisionListParams, (offset, Some(5))), "offset=5")]
    #[case(generate!(TemplateRevisionListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(TemplateRevisionListParams, (orderby, Some(WpApiParamTemplateRevisionsOrderBy::Slug))), "orderby=slug")]
    fn list_template_revisions(
        endpoint: TemplateRevisionsRequestEndpoint,
        #[case] params: TemplateRevisionListParams,
        #[case] expected_additional_params: &str,
    ) {
        let template_id = TemplateId("foo".to_string());
        let expected_path = |context: &str| {
            if expected_additional_params.is_empty() {
                format!("/templates/foo/revisions?context={context}")
            } else {
                format!("/templates/foo/revisions?context={context}&{expected_additional_params}")
            }
        };
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&template_id, &params),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(&template_id, &params),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(&template_id, &params),
            &expected_path("view"),
        );
    }

    #[rstest]
    #[case(TemplateRevisionListParams::default(), &[], "/templates/foo/revisions?context=view&_fields=")]
    #[case(generate!(TemplateRevisionListParams, (page, Some(1))), &[SparseTemplateRevisionFieldWithViewContext::Author], "/templates/foo/revisions?context=view&page=1&_fields=author")]
    fn filter_list_template_revisions_with_view_context(
        endpoint: TemplateRevisionsRequestEndpoint,
        #[case] params: TemplateRevisionListParams,
        #[case] fields: &[SparseTemplateRevisionFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_view_context(&TemplateId("foo".to_string()), &params, fields),
            expected_path,
        );
    }

    #[rstest]
    fn retrieve_template_revision(endpoint: TemplateRevisionsRequestEndpoint) {
        let template_id = TemplateId("foo".to_string());
        let revision_id = TemplateRevisionId(42);
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&template_id, &revision_id),
            "/templates/foo/revisions/42?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&template_id, &revision_id),
            "/templates/foo/revisions/42?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&template_id, &revision_id),
            "/templates/foo/revisions/42?context=view",
        );
    }

    #[rstest]
    #[case(&[], "/templates/foo/revisions/42?context=view&_fields=")]
    #[case(&[SparseTemplateRevisionFieldWithViewContext::Slug], "/templates/foo/revisions/42?context=view&_fields=slug")]
    fn filter_retrieve_template_revision_with_view_context(
        endpoint: TemplateRevisionsRequestEndpoint,
        #[case] fields: &[SparseTemplateRevisionFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(
                &TemplateId("foo".to_string()),
                &TemplateRevisionId(42),
                fields,
            ),
            expected_path,
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> TemplateRevisionsRequestEndpoint {
        TemplateRevisionsRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
