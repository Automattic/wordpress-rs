use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::menu_locations::MenuLocation;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum MenuLocationsRequest {
    #[contextual_get(url = "/menu-locations", output = crate::menu_locations::SparseMenuLocationsResponse)]
    List,
    #[contextual_get(url = "/menu-locations/<menu_location>", output = crate::menu_locations::SparseMenuLocation)]
    Retrieve,
}

impl DerivedRequest for MenuLocationsRequest {
    fn namespace() -> impl AsNamespace {
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
    fn list_menu_locations(endpoint: MenuLocationsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(),
            "/menu-locations?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(),
            "/menu-locations?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(),
            "/menu-locations?context=view",
        );
    }

    #[rstest]
    fn retrieve_menu_location(endpoint: MenuLocationsRequestEndpoint) {
        let location = &MenuLocation("primary".to_string());
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(location),
            "/menu-locations/primary?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(location),
            "/menu-locations/primary?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(location),
            "/menu-locations/primary?context=view",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> MenuLocationsRequestEndpoint {
        MenuLocationsRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
