use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::nav_menu_items::NavMenuItemId;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum NavMenuItemsRequest {
    #[contextual_get(url = "/menu-items", params = &crate::nav_menu_items::NavMenuItemListParams, output = Vec<crate::nav_menu_items::SparseNavMenuItem>, filter_by = crate::nav_menu_items::SparseNavMenuItemField)]
    List,
    #[contextual_get(url = "/menu-items/<nav_menu_item_id>", output = crate::nav_menu_items::SparseNavMenuItem, filter_by = crate::nav_menu_items::SparseNavMenuItemField)]
    Retrieve,
    #[post(url = "/menu-items", params = &crate::nav_menu_items::NavMenuItemCreateParams, output = crate::nav_menu_items::NavMenuItemWithEditContext)]
    Create,
    #[delete(url = "/menu-items/<nav_menu_item_id>", output = crate::nav_menu_items::NavMenuItemDeleteResponse)]
    Delete,
    #[post(url = "/menu-items/<nav_menu_item_id>", params = &crate::nav_menu_items::NavMenuItemUpdateParams, output = crate::nav_menu_items::NavMenuItemWithEditContext)]
    Update,
}

impl DerivedRequest for NavMenuItemsRequest {
    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        match self {
            // Trashing a nav menu item is not supported:
            // https://github.com/WordPress/WordPress/blob/b27e369cb25445784bc014d6fa731558beaf320d/wp-includes/rest-api/endpoints/class-wp-rest-menu-items-controller.php#L298-L302
            NavMenuItemsRequest::Delete => vec![("force", true.to_string())],
            _ => vec![],
        }
    }

    fn namespace() -> impl AsNamespace {
        WpNamespace::WpV2
    }
}
