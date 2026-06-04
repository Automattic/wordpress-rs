use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::global_styles::GlobalStylesId;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum GlobalStylesRequest {
    #[contextual_get(url = "/global-styles/<global_styles_id>", output = crate::global_styles::SparseGlobalStyles, filter_by = crate::global_styles::SparseGlobalStylesField)]
    Retrieve,
}

impl DerivedRequest for GlobalStylesRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::endpoint::{
        ApiUrlResolver,
        tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn retrieve_global_styles(endpoint: GlobalStylesRequestEndpoint) {
        let id = GlobalStylesId(42);
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&id),
            "/global-styles/42?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&id),
            "/global-styles/42?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&id),
            "/global-styles/42?context=view",
        );
    }

    #[rstest]
    fn filter_retrieve_global_styles(endpoint: GlobalStylesRequestEndpoint) {
        use crate::global_styles::SparseGlobalStylesFieldWithViewContext;
        let id = GlobalStylesId(42);
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(
                &id,
                &[
                    SparseGlobalStylesFieldWithViewContext::Id,
                    SparseGlobalStylesFieldWithViewContext::Title,
                ],
            ),
            "/global-styles/42?context=view&_fields=id%2Ctitle",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> GlobalStylesRequestEndpoint {
        GlobalStylesRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
