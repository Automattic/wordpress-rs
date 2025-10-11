use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::navigations::{
    NavigationId, NavigationListParams, NavigationUpdateParams, NavigationWithEditContext,
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum NavigationsRequest {
    #[contextual_paged(url = "/navigation", params = &NavigationListParams, output = Vec<crate::navigations::SparseNavigation>, filter_by = crate::navigations::SparseNavigationField)]
    List,
    #[contextual_get(url = "/navigation/<navigation_id>", params = &crate::navigations::NavigationRetrieveParams, output = crate::navigations::SparseNavigation, filter_by = crate::navigations::SparseNavigationField)]
    Retrieve,
    #[post(url = "/navigation", params = &crate::navigations::NavigationCreateParams, output = crate::navigations::NavigationWithEditContext)]
    Create,
    #[delete(url = "/navigation/<navigation_id>", output = crate::navigations::NavigationDeleteResponse)]
    Delete,
    #[delete(url = "/navigation/<navigation_id>", output = crate::navigations::NavigationWithEditContext)]
    Trash,
    #[post(url = "/navigation/<navigation_id>", params = &NavigationUpdateParams, output = NavigationWithEditContext)]
    Update,
}

impl DerivedRequest for NavigationsRequest {
    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        match self {
            NavigationsRequest::Delete => vec![("force", true.to_string())],
            NavigationsRequest::Trash => vec![("force", false.to_string())],
            _ => vec![],
        }
    }

    fn namespace() -> impl AsNamespace {
        WpNamespace::WpV2
    }
}
