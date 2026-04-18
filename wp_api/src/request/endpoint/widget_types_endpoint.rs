use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::widget_types::WidgetTypeId;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum WidgetTypesRequest {
    #[contextual_get(url = "/widget-types", output = Vec<crate::widget_types::SparseWidgetType>, filter_by = crate::widget_types::SparseWidgetTypeField)]
    List,
    #[contextual_get(url = "/widget-types/<widget_type_id>", output = crate::widget_types::SparseWidgetType, filter_by = crate::widget_types::SparseWidgetTypeField)]
    Retrieve,
}

impl DerivedRequest for WidgetTypesRequest {
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
        widget_types::{
            SparseWidgetTypeFieldWithEditContext, SparseWidgetTypeFieldWithEmbedContext,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn list_widget_types(endpoint: WidgetTypesRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(),
            "/widget-types?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(),
            "/widget-types?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(),
            "/widget-types?context=view",
        );
    }

    #[rstest]
    #[case(&[], "/widget-types?context=edit&_fields=")]
    #[case(&[SparseWidgetTypeFieldWithEditContext::Description], "/widget-types?context=edit&_fields=description")]
    #[case(ALL_SPARSE_WIDGET_TYPE_FIELDS_WITH_EDIT_CONTEXT, &format!("/widget-types?context=edit&{EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_WIDGET_TYPE_FIELDS_WITH_EDIT_CONTEXT}"))]
    fn filter_list_widget_types(
        endpoint: WidgetTypesRequestEndpoint,
        #[case] fields: &[SparseWidgetTypeFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(fields),
            expected_path,
        );
    }

    #[rstest]
    fn retrieve_widget_type(endpoint: WidgetTypesRequestEndpoint) {
        let widget_type = &WidgetTypeId("calendar".to_string());
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(widget_type),
            "/widget-types/calendar?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(widget_type),
            "/widget-types/calendar?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(widget_type),
            "/widget-types/calendar?context=view",
        );
    }

    #[rstest]
    fn filter_retrieve_widget_type_with_embed_context(endpoint: WidgetTypesRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_embed_context(
                &WidgetTypeId("recent-posts".to_string()),
                &[
                    SparseWidgetTypeFieldWithEmbedContext::Name,
                    SparseWidgetTypeFieldWithEmbedContext::IsMulti,
                ],
            ),
            "/widget-types/recent-posts?context=embed&_fields=name%2Cis_multi",
        );
    }

    const EXPECTED_QUERY_PAIRS_FOR_ALL_SPARSE_WIDGET_TYPE_FIELDS_WITH_EDIT_CONTEXT: &str =
        "_fields=id%2Cname%2Cdescription%2Cis_multi%2Cclassname";
    const ALL_SPARSE_WIDGET_TYPE_FIELDS_WITH_EDIT_CONTEXT: &[SparseWidgetTypeFieldWithEditContext;
         5] = &[
        SparseWidgetTypeFieldWithEditContext::Id,
        SparseWidgetTypeFieldWithEditContext::Name,
        SparseWidgetTypeFieldWithEditContext::Description,
        SparseWidgetTypeFieldWithEditContext::IsMulti,
        SparseWidgetTypeFieldWithEditContext::Classname,
    ];

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> WidgetTypesRequestEndpoint {
        WidgetTypesRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
