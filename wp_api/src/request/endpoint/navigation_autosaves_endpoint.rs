use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{navigation_revisions::NavigationRevisionId, navigations::NavigationId};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum NavigationAutosavesRequest {
    #[contextual_get(url = "/navigation/<navigation_id>/autosaves", output = Vec<crate::navigation_revisions::SparseNavigationRevision>, filter_by = crate::navigation_revisions::SparseNavigationRevisionField)]
    List,
    #[contextual_get(url = "/navigation/<navigation_id>/autosaves/<navigation_revision_id>", output = crate::navigation_revisions::SparseNavigationRevision, filter_by = crate::navigation_revisions::SparseNavigationRevisionField)]
    Retrieve,
    #[post(url = "/navigation/<navigation_id>/autosaves", params = &crate::navigations::NavigationCreateParams, output = crate::navigation_revisions::NavigationRevisionWithEditContext)]
    Create,
}

impl DerivedRequest for NavigationAutosavesRequest {
    fn namespace() -> impl AsNamespace {
        WpNamespace::WpV2
    }
}
