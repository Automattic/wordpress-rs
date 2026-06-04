use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{
    global_styles::GlobalStylesId,
    global_styles_revisions::{GlobalStylesRevisionId, GlobalStylesRevisionListParams},
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum GlobalStylesRevisionsRequest {
    #[contextual_paged(url = "/global-styles/<global_styles_id>/revisions", params = &GlobalStylesRevisionListParams, output = Vec<crate::global_styles_revisions::SparseGlobalStylesRevision>, filter_by = crate::global_styles_revisions::SparseGlobalStylesRevisionField)]
    List,
    #[contextual_get(url = "/global-styles/<global_styles_id>/revisions/<global_styles_revision_id>", output = crate::global_styles_revisions::SparseGlobalStylesRevision, filter_by = crate::global_styles_revisions::SparseGlobalStylesRevisionField)]
    Retrieve,
}

impl DerivedRequest for GlobalStylesRevisionsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        global_styles::GlobalStylesId,
        global_styles_revisions::{GlobalStylesRevisionId, GlobalStylesRevisionListParams},
        request::endpoint::{
            ApiUrlResolver,
            tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn list_global_styles_revisions_with_default_params(
        endpoint: GlobalStylesRevisionsRequestEndpoint,
    ) {
        let global_styles_id = GlobalStylesId(42);
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(
                &global_styles_id,
                &GlobalStylesRevisionListParams::default(),
            ),
            "/global-styles/42/revisions?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(
                &global_styles_id,
                &GlobalStylesRevisionListParams::default(),
            ),
            "/global-styles/42/revisions?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(
                &global_styles_id,
                &GlobalStylesRevisionListParams::default(),
            ),
            "/global-styles/42/revisions?context=view",
        );
    }

    #[rstest]
    fn list_global_styles_revisions_with_params(endpoint: GlobalStylesRevisionsRequestEndpoint) {
        let global_styles_id = GlobalStylesId(42);
        let params = GlobalStylesRevisionListParams {
            page: Some(2),
            per_page: Some(5),
            offset: Some(10),
        };
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&global_styles_id, &params),
            "/global-styles/42/revisions?context=edit&page=2&per_page=5&offset=10",
        );
    }

    #[rstest]
    fn retrieve_global_styles_revision(endpoint: GlobalStylesRevisionsRequestEndpoint) {
        let global_styles_id = GlobalStylesId(42);
        let revision_id = GlobalStylesRevisionId(99);
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&global_styles_id, &revision_id),
            "/global-styles/42/revisions/99?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&global_styles_id, &revision_id),
            "/global-styles/42/revisions/99?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&global_styles_id, &revision_id),
            "/global-styles/42/revisions/99?context=view",
        );
    }

    #[rstest]
    fn filter_list_global_styles_revisions(endpoint: GlobalStylesRevisionsRequestEndpoint) {
        use crate::global_styles_revisions::SparseGlobalStylesRevisionFieldWithEditContext;
        let global_styles_id = GlobalStylesId(42);
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(
                &global_styles_id,
                &GlobalStylesRevisionListParams::default(),
                &[
                    SparseGlobalStylesRevisionFieldWithEditContext::Id,
                    SparseGlobalStylesRevisionFieldWithEditContext::Author,
                ],
            ),
            "/global-styles/42/revisions?context=edit&_fields=id%2Cauthor",
        );
    }

    #[rstest]
    fn filter_retrieve_global_styles_revision(endpoint: GlobalStylesRevisionsRequestEndpoint) {
        use crate::global_styles_revisions::SparseGlobalStylesRevisionFieldWithViewContext;
        let global_styles_id = GlobalStylesId(42);
        let revision_id = GlobalStylesRevisionId(99);
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(
                &global_styles_id,
                &revision_id,
                &[
                    SparseGlobalStylesRevisionFieldWithViewContext::Id,
                    SparseGlobalStylesRevisionFieldWithViewContext::Date,
                ],
            ),
            "/global-styles/42/revisions/99?context=view&_fields=id%2Cdate",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> GlobalStylesRevisionsRequestEndpoint {
        GlobalStylesRevisionsRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
