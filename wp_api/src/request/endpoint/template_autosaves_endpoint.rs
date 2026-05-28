use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{template_autosaves::TemplateAutosaveId, templates::TemplateId};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum TemplateAutosavesRequest {
    #[contextual_get(url = "/templates/<template_id>/autosaves", output = Vec<crate::template_autosaves::SparseTemplateAutosave>, filter_by = crate::template_autosaves::SparseTemplateAutosaveField)]
    List,
    #[contextual_get(url = "/templates/<template_id>/autosaves/<template_autosave_id>", output = crate::template_autosaves::SparseTemplateAutosave, filter_by = crate::template_autosaves::SparseTemplateAutosaveField)]
    Retrieve,
    #[post(url = "/templates/<template_id>/autosaves", params = &crate::templates::TemplateCreateParams, output = crate::template_autosaves::TemplateAutosaveWithEditContext)]
    Create,
}

impl DerivedRequest for TemplateAutosavesRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::{
            ApiUrlResolver,
            tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
        },
        template_autosaves::{SparseTemplateAutosaveFieldWithViewContext, TemplateAutosaveId},
        templates::TemplateId,
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn list_template_autosaves(endpoint: TemplateAutosavesRequestEndpoint) {
        let template_id = TemplateId("foo".to_string());
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&template_id),
            "/templates/foo/autosaves?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(&template_id),
            "/templates/foo/autosaves?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(&template_id),
            "/templates/foo/autosaves?context=view",
        );
    }

    #[rstest]
    #[case(&[], "/templates/foo/autosaves?context=view&_fields=")]
    #[case(&[SparseTemplateAutosaveFieldWithViewContext::Author], "/templates/foo/autosaves?context=view&_fields=author")]
    fn filter_list_template_autosaves_with_view_context(
        endpoint: TemplateAutosavesRequestEndpoint,
        #[case] fields: &[SparseTemplateAutosaveFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_view_context(&TemplateId("foo".to_string()), fields),
            expected_path,
        );
    }

    #[rstest]
    fn retrieve_template_autosave(endpoint: TemplateAutosavesRequestEndpoint) {
        let template_id = TemplateId("foo".to_string());
        let autosave_id = TemplateAutosaveId(42);
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&template_id, &autosave_id),
            "/templates/foo/autosaves/42?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&template_id, &autosave_id),
            "/templates/foo/autosaves/42?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&template_id, &autosave_id),
            "/templates/foo/autosaves/42?context=view",
        );
    }

    #[rstest]
    #[case(&[], "/templates/foo/autosaves/42?context=view&_fields=")]
    #[case(&[SparseTemplateAutosaveFieldWithViewContext::Slug], "/templates/foo/autosaves/42?context=view&_fields=slug")]
    fn filter_retrieve_template_autosave_with_view_context(
        endpoint: TemplateAutosavesRequestEndpoint,
        #[case] fields: &[SparseTemplateAutosaveFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(
                &TemplateId("foo".to_string()),
                &TemplateAutosaveId(42),
                fields,
            ),
            expected_path,
        );
    }

    #[rstest]
    fn create_template_autosave(endpoint: TemplateAutosavesRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.create(&TemplateId("foo".to_string())),
            "/templates/foo/autosaves",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> TemplateAutosavesRequestEndpoint {
        TemplateAutosavesRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
