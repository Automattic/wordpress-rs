use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{template_part_autosaves::TemplatePartAutosaveId, template_parts::TemplatePartId};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum TemplatePartAutosavesRequest {
    #[contextual_get(url = "/template-parts/<template_part_id>/autosaves", output = Vec<crate::template_part_autosaves::SparseTemplatePartAutosave>, filter_by = crate::template_part_autosaves::SparseTemplatePartAutosaveField)]
    List,
    #[contextual_get(url = "/template-parts/<template_part_id>/autosaves/<template_part_autosave_id>", output = crate::template_part_autosaves::SparseTemplatePartAutosave, filter_by = crate::template_part_autosaves::SparseTemplatePartAutosaveField)]
    Retrieve,
    #[post(url = "/template-parts/<template_part_id>/autosaves", params = &crate::template_parts::TemplatePartCreateParams, output = crate::template_part_autosaves::TemplatePartAutosaveWithEditContext)]
    Create,
}

impl DerivedRequest for TemplatePartAutosavesRequest {
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
        template_part_autosaves::{
            SparseTemplatePartAutosaveFieldWithViewContext, TemplatePartAutosaveId,
        },
        template_parts::TemplatePartId,
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn list_template_part_autosaves(endpoint: TemplatePartAutosavesRequestEndpoint) {
        let template_part_id = TemplatePartId("foo".to_string());
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&template_part_id),
            "/template-parts/foo/autosaves?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(&template_part_id),
            "/template-parts/foo/autosaves?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(&template_part_id),
            "/template-parts/foo/autosaves?context=view",
        );
    }

    #[rstest]
    #[case(&[], "/template-parts/foo/autosaves?context=view&_fields=")]
    #[case(&[SparseTemplatePartAutosaveFieldWithViewContext::Author], "/template-parts/foo/autosaves?context=view&_fields=author")]
    fn filter_list_template_part_autosaves_with_view_context(
        endpoint: TemplatePartAutosavesRequestEndpoint,
        #[case] fields: &[SparseTemplatePartAutosaveFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_view_context(&TemplatePartId("foo".to_string()), fields),
            expected_path,
        );
    }

    #[rstest]
    fn retrieve_template_part_autosave(endpoint: TemplatePartAutosavesRequestEndpoint) {
        let template_part_id = TemplatePartId("foo".to_string());
        let autosave_id = TemplatePartAutosaveId(42);
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&template_part_id, &autosave_id),
            "/template-parts/foo/autosaves/42?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&template_part_id, &autosave_id),
            "/template-parts/foo/autosaves/42?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&template_part_id, &autosave_id),
            "/template-parts/foo/autosaves/42?context=view",
        );
    }

    #[rstest]
    #[case(&[], "/template-parts/foo/autosaves/42?context=view&_fields=")]
    #[case(&[SparseTemplatePartAutosaveFieldWithViewContext::Slug], "/template-parts/foo/autosaves/42?context=view&_fields=slug")]
    fn filter_retrieve_template_part_autosave_with_view_context(
        endpoint: TemplatePartAutosavesRequestEndpoint,
        #[case] fields: &[SparseTemplatePartAutosaveFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(
                &TemplatePartId("foo".to_string()),
                &TemplatePartAutosaveId(42),
                fields,
            ),
            expected_path,
        );
    }

    #[rstest]
    fn create_template_part_autosave(endpoint: TemplatePartAutosavesRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.create(&TemplatePartId("foo".to_string())),
            "/template-parts/foo/autosaves",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> TemplatePartAutosavesRequestEndpoint {
        TemplatePartAutosavesRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
