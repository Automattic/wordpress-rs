use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace,
        reader_posts::ReaderTagSlug,
        reader_tags::{ReaderInterestsResponse, ReaderMenu, ReaderTagResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum ReaderTagsRequest {
    // The Reader menu: its streams, the tags the user follows, and tags to suggest. Answers
    // signed out, with Discover and the suggested tags only.
    #[get(url = "/read/menu", output = ReaderMenu)]
    Menu,
    // A tag, by its slug. Answers signed out. A slug that names no tag is a 404 `invalid_tag`.
    #[get(url = "/read/tags/<reader_tag_slug>", output = ReaderTagResponse)]
    Tag,
    // The tags a new reader picks from. Answers signed out.
    #[get(url = "/read/interests", output = ReaderInterestsResponse)]
    Interests,
}

impl DerivedRequest for ReaderTagsRequest {
    fn namespace(&self) -> impl AsNamespace {
        match self {
            // The newest menu. It has the user's organizations, which `rest/v1.2` calls teams.
            Self::Menu => WpComNamespace::RestV1_3,
            // The newest tag info: `rest/v1.3` has none.
            Self::Tag => WpComNamespace::RestV1_2,
            Self::Interests => WpComNamespace::V2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::ApiUrlResolver,
        wp_com::endpoint::tests::{
            fixture_wp_com_api_url_resolver, validate_wp_com_rest_v1_2_endpoint,
            validate_wp_com_rest_v1_3_endpoint, validate_wp_com_v2_endpoint,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn menu(endpoint: ReaderTagsRequestEndpoint) {
        validate_wp_com_rest_v1_3_endpoint(endpoint.menu(), "/read/menu");
    }

    #[rstest]
    fn tag(endpoint: ReaderTagsRequestEndpoint) {
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.tag(&ReaderTagSlug("photography".to_string())),
            "/read/tags/photography",
        );
    }

    /// A tag's slug can hold any character, such as Japanese, so it's escaped in the path.
    #[rstest]
    fn tag_escapes_the_slug(endpoint: ReaderTagsRequestEndpoint) {
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.tag(&ReaderTagSlug("写真".to_string())),
            "/read/tags/%E5%86%99%E7%9C%9F",
        );
    }

    #[rstest]
    fn interests(endpoint: ReaderTagsRequestEndpoint) {
        validate_wp_com_v2_endpoint(endpoint.interests(), "/read/interests");
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> ReaderTagsRequestEndpoint {
        ReaderTagsRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
