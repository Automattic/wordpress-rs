use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{
    navigation_revisions::{NavigationRevisionId, NavigationRevisionListParams},
    navigations::NavigationId,
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum NavigationRevisionsRequest {
    #[contextual_paged(url = "/navigation/<navigation_id>/revisions", params = &NavigationRevisionListParams, output = Vec<crate::navigation_revisions::SparseNavigationRevision>, filter_by = crate::navigation_revisions::SparseNavigationRevisionField)]
    List,
    #[contextual_get(url = "/navigation/<navigation_id>/revisions/<navigation_revision_id>", output = crate::navigation_revisions::SparseNavigationRevision, filter_by = crate::navigation_revisions::SparseNavigationRevisionField)]
    Retrieve,
    #[delete(url = "/navigation/<navigation_id>/revisions/<navigation_revision_id>", output = crate::navigation_revisions::NavigationRevisionDeleteResponse)]
    Delete,
}

impl DerivedRequest for NavigationRevisionsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::WpV2
    }

    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        match self {
            NavigationRevisionsRequest::Delete => vec![("force", "true".to_string())],
            _ => vec![],
        }
    }
}
