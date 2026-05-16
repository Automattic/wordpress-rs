use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::sidebars::SidebarId;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum SidebarsRequest {
    #[contextual_get(url = "/sidebars", output = Vec<crate::sidebars::SparseSidebar>, filter_by = crate::sidebars::SparseSidebarField)]
    List,
    #[contextual_get(url = "/sidebars/<sidebar_id>", output = crate::sidebars::SparseSidebar, filter_by = crate::sidebars::SparseSidebarField)]
    Retrieve,
    #[post(url = "/sidebars/<sidebar_id>", params = &crate::sidebars::SidebarUpdateParams, output = crate::sidebars::SidebarWithEditContext)]
    Update,
}

impl DerivedRequest for SidebarsRequest {
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
    fn list_sidebars(endpoint: SidebarsRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.list_with_edit_context(), "/sidebars?context=edit");
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(),
            "/sidebars?context=embed",
        );
        validate_wp_v2_endpoint(endpoint.list_with_view_context(), "/sidebars?context=view");
    }

    #[rstest]
    fn retrieve_sidebar(endpoint: SidebarsRequestEndpoint) {
        let sidebar_id = &SidebarId("sidebar-1".to_string());
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(sidebar_id),
            "/sidebars/sidebar-1?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(sidebar_id),
            "/sidebars/sidebar-1?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(sidebar_id),
            "/sidebars/sidebar-1?context=view",
        );
    }

    #[rstest]
    fn update_sidebar(endpoint: SidebarsRequestEndpoint) {
        let sidebar_id = &SidebarId("sidebar-1".to_string());
        validate_wp_v2_endpoint(endpoint.update(sidebar_id), "/sidebars/sidebar-1");
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> SidebarsRequestEndpoint {
        SidebarsRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
