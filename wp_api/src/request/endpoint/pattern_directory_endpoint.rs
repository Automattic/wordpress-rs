use super::{AsNamespace, DerivedRequest, WpNamespace};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum PatternDirectoryRequest {
    #[contextual_get(url = "/pattern-directory/patterns", params = &crate::pattern_directory::PatternDirectoryListParams, output = Vec<crate::pattern_directory::SparsePatternDirectoryItem>, filter_by = crate::pattern_directory::SparsePatternDirectoryItemField)]
    List,
}

impl DerivedRequest for PatternDirectoryRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        pattern_directory::{
            PatternDirectoryListParams, SparsePatternDirectoryItemFieldWithViewContext,
        },
        request::endpoint::{
            ApiUrlResolver,
            tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn list_pattern_directory(endpoint: PatternDirectoryRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&PatternDirectoryListParams::default()),
            "/pattern-directory/patterns?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(&PatternDirectoryListParams::default()),
            "/pattern-directory/patterns?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(&PatternDirectoryListParams::default()),
            "/pattern-directory/patterns?context=view",
        );
    }

    #[rstest]
    fn list_pattern_directory_with_params(endpoint: PatternDirectoryRequestEndpoint) {
        let params = PatternDirectoryListParams {
            per_page: Some(10),
            category: Some(crate::pattern_directory::PatternDirectoryCategoryId(5)),
            ..Default::default()
        };
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(&params),
            "/pattern-directory/patterns?context=view&per_page=10&category=5",
        );
    }

    #[rstest]
    fn filter_list_pattern_directory(endpoint: PatternDirectoryRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_view_context(
                &PatternDirectoryListParams::default(),
                &[
                    SparsePatternDirectoryItemFieldWithViewContext::Id,
                    SparsePatternDirectoryItemFieldWithViewContext::Title,
                ],
            ),
            "/pattern-directory/patterns?context=view&_fields=id%2Ctitle",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> PatternDirectoryRequestEndpoint {
        PatternDirectoryRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}
