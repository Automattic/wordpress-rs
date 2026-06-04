use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{
    post_revisions::PostRevisionId, posts::PostId,
    request::endpoint::posts_endpoint::PostEndpointType,
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum AutosavesRequest {
    #[contextual_get(url = "/<post_endpoint_type>/<post_id>/autosaves", output = Vec<crate::post_revisions::SparseAnyPostRevision>, filter_by = crate::post_revisions::SparseAnyPostRevisionField)]
    List,
    #[contextual_get(url = "/<post_endpoint_type>/<post_id>/autosaves/<post_revision_id>", output = crate::post_revisions::SparseAnyPostRevision, filter_by = crate::post_revisions::SparseAnyPostRevisionField)]
    Retrieve,
    #[post(url = "/<post_endpoint_type>/<post_id>/autosaves", params = &crate::posts::PostCreateParams, output = crate::post_revisions::AnyPostRevisionWithEditContext)]
    Create,
}

impl DerivedRequest for AutosavesRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::endpoint::ApiUrlResolver;
    use crate::{
        posts::PostId,
        request::endpoint::tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn list_autosaves(endpoint: AutosavesRequestEndpoint) {
        let post_id = PostId(777);
        let expected_path = |context: &str| format!("/posts/{post_id}/autosaves?context={context}");
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&PostEndpointType::Posts, &post_id),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(&PostEndpointType::Posts, &post_id),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(&PostEndpointType::Posts, &post_id),
            &expected_path("view"),
        );
    }

    #[rstest]
    fn retrieve_autosave(endpoint: AutosavesRequestEndpoint) {
        let post_id = PostId(777);
        let post_revision_id = PostRevisionId(888);
        let expected_path = |context: &str| {
            format!("/posts/{post_id}/autosaves/{post_revision_id}?context={context}")
        };
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(
                &PostEndpointType::Posts,
                &post_id,
                &post_revision_id,
            ),
            &expected_path("edit"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(
                &PostEndpointType::Posts,
                &post_id,
                &post_revision_id,
            ),
            &expected_path("embed"),
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(
                &PostEndpointType::Posts,
                &post_id,
                &post_revision_id,
            ),
            &expected_path("view"),
        );
    }

    #[rstest]
    fn create_autosave(endpoint: AutosavesRequestEndpoint) {
        let post_id = PostId(777);
        let expected_path = format!("/posts/{post_id}/autosaves");

        validate_wp_v2_endpoint(
            endpoint.create(&PostEndpointType::Posts, &post_id),
            &expected_path,
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> AutosavesRequestEndpoint {
        AutosavesRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
