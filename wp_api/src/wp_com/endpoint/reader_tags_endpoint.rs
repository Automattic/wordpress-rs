use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace,
        reader_posts::ReaderTagSlug,
        reader_tags::{
            ReaderFollowTagsParams, ReaderFollowTagsResponse, ReaderInterestsParams,
            ReaderInterestsResponse, ReaderMenu, ReaderMenuParams, ReaderTagResponse,
            ReaderUnfollowTagResponse,
        },
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum ReaderTagsRequest {
    // The Reader menu: its streams, the tags the user follows, and tags to suggest. Answers
    // signed out, with Discover and the suggested tags only.
    #[get(url = "/read/menu", params = &ReaderMenuParams, output = ReaderMenu)]
    Menu,
    // A tag, by its slug. Answers signed out. A slug that names no tag is a 404 `invalid_tag`.
    #[get(url = "/read/tags/<reader_tag_slug>", output = ReaderTagResponse)]
    Tag,
    // The tags a new reader picks from. Answers signed out.
    #[get(url = "/read/interests", params = &ReaderInterestsParams, output = ReaderInterestsResponse)]
    Interests,
    // Follows one or more tags, and answers with every tag the user now follows. For a tag the
    // user already follows it's still a 200, with nothing in `added_tags`.
    #[post(url = "/read/tags/mine/new", params = &ReaderFollowTagsParams, output = ReaderFollowTagsResponse)]
    FollowTags,
    // Unfollows a tag, by its slug, and answers with every tag the user still follows. It takes
    // no parameters. For a tag the user doesn't follow it's a 400 `not_subscribed`.
    #[post(url = "/read/tags/<reader_tag_slug>/mine/delete", output = ReaderUnfollowTagResponse)]
    UnfollowTag,
}

impl DerivedRequest for ReaderTagsRequest {
    fn namespace(&self) -> impl AsNamespace {
        match self {
            // The newest menu. It has the user's organizations, which `rest/v1.2` calls teams.
            Self::Menu => WpComNamespace::RestV1_3,
            // The newest tag info: `rest/v1.3` has none.
            Self::Tag => WpComNamespace::RestV1_2,
            // The newest follow, which takes several tags. `rest/v1.1` follows one, named in the
            // address.
            Self::FollowTags => WpComNamespace::RestV1_2,
            // The only version: `rest/v1.2` has none.
            Self::UnfollowTag => WpComNamespace::RestV1_1,
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
            fixture_wp_com_api_url_resolver, validate_wp_com_rest_v1_1_endpoint,
            validate_wp_com_rest_v1_2_endpoint, validate_wp_com_rest_v1_3_endpoint,
            validate_wp_com_v2_endpoint,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn menu(endpoint: ReaderTagsRequestEndpoint) {
        validate_wp_com_rest_v1_3_endpoint(
            endpoint.menu(&ReaderMenuParams::default()),
            "/read/menu?",
        );
        validate_wp_com_rest_v1_3_endpoint(
            endpoint.menu(&ReaderMenuParams {
                locale: Some("pt-br".to_string()),
            }),
            "/read/menu?locale=pt-br",
        );
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
        validate_wp_com_v2_endpoint(
            endpoint.interests(&ReaderInterestsParams::default()),
            "/read/interests?",
        );
        validate_wp_com_v2_endpoint(
            endpoint.interests(&ReaderInterestsParams {
                locale: Some("zh-tw".to_string()),
            }),
            "/read/interests?_locale=zh-tw",
        );
    }

    #[rstest]
    fn follow_tags(endpoint: ReaderTagsRequestEndpoint) {
        validate_wp_com_rest_v1_2_endpoint(endpoint.follow_tags(), "/read/tags/mine/new");
    }

    #[rstest]
    fn unfollow_tag(endpoint: ReaderTagsRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.unfollow_tag(&ReaderTagSlug("photography".to_string())),
            "/read/tags/photography/mine/delete",
        );
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.unfollow_tag(&ReaderTagSlug("写真".to_string())),
            "/read/tags/%E5%86%99%E7%9C%9F/mine/delete",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> ReaderTagsRequestEndpoint {
        ReaderTagsRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
