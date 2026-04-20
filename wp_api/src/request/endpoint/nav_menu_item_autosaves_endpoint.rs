use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::nav_menu_item_revisions::NavMenuItemRevisionId;
use crate::nav_menu_items::NavMenuItemId;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum NavMenuItemAutosavesRequest {
    #[contextual_get(url = "/menu-items/<nav_menu_item_id>/autosaves", output = Vec<crate::nav_menu_item_revisions::SparseNavMenuItemRevision>, filter_by = crate::nav_menu_item_revisions::SparseNavMenuItemRevisionField)]
    List,
    #[contextual_get(url = "/menu-items/<nav_menu_item_id>/autosaves/<nav_menu_item_revision_id>", output = crate::nav_menu_item_revisions::SparseNavMenuItemRevision, filter_by = crate::nav_menu_item_revisions::SparseNavMenuItemRevisionField)]
    Retrieve,
    #[post(url = "/menu-items/<nav_menu_item_id>/autosaves", params = &crate::nav_menu_item_revisions::NavMenuItemRevisionCreateParams, output = crate::nav_menu_item_revisions::NavMenuItemRevisionWithEditContext)]
    Create,
}

impl DerivedRequest for NavMenuItemAutosavesRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::endpoint::ApiUrlResolver;
    use crate::{
        nav_menu_item_revisions::NavMenuItemRevisionId,
        nav_menu_items::NavMenuItemId,
        request::endpoint::tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn list_autosaves(endpoint: NavMenuItemAutosavesRequestEndpoint) {
        let nav_menu_item_id = NavMenuItemId(777);
        let expected_path =
            |context: &str| format!("/menu-items/{nav_menu_item_id}/autosaves?context={context}");
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&nav_menu_item_id),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(&nav_menu_item_id),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(&nav_menu_item_id),
            &expected_path("view"),
        );
    }

    #[rstest]
    fn retrieve_autosave(endpoint: NavMenuItemAutosavesRequestEndpoint) {
        let nav_menu_item_id = NavMenuItemId(777);
        let nav_menu_item_revision_id = NavMenuItemRevisionId(888);
        let expected_path = |context: &str| {
            format!(
                "/menu-items/{nav_menu_item_id}/autosaves/{nav_menu_item_revision_id}?context={context}"
            )
        };
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&nav_menu_item_id, &nav_menu_item_revision_id),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&nav_menu_item_id, &nav_menu_item_revision_id),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&nav_menu_item_id, &nav_menu_item_revision_id),
            &expected_path("view"),
        );
    }

    #[rstest]
    fn create_autosave(endpoint: NavMenuItemAutosavesRequestEndpoint) {
        let nav_menu_item_id = NavMenuItemId(777);
        let expected_path = format!("/menu-items/{nav_menu_item_id}/autosaves");

        validate_wp_v2_endpoint(endpoint.create(&nav_menu_item_id), &expected_path);
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> NavMenuItemAutosavesRequestEndpoint {
        NavMenuItemAutosavesRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
