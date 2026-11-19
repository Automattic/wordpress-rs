use crate::{
    posts::PostId,
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        reader_likes::{ReaderLikesParams, ReaderLikesResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum ReaderLikesRequest {
    // The users who liked a post, newest like first, paged by the date of the last like. Answers
    // signed out for a public site; a private site's is then a 403 `unauthorized`. A post that
    // doesn't exist is a 404 `unknown_post`.
    #[get(url = "/sites/<wp_com_site_id>/posts/<post_id>/likes", params = &ReaderLikesParams, output = ReaderLikesResponse)]
    PostLikes,
}

impl DerivedRequest for ReaderLikesRequest {
    fn namespace(&self) -> impl AsNamespace {
        // The newest list of likes, and the one with each like's date, which paging needs.
        // `rest/v1.1` returns 90 likes with no dates and no way to the rest.
        WpComNamespace::RestV1_2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::ApiUrlResolver,
        wp_com::{
            endpoint::tests::{
                fixture_wp_com_api_url_resolver, validate_wp_com_rest_v1_2_endpoint,
            },
            me::WpComUserId,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    fn all_params() -> ReaderLikesParams {
        ReaderLikesParams {
            number: Some(90),
            before: Some("2026-11-01T00:00:00Z".parse().unwrap()),
            exclude: vec![WpComUserId(11), WpComUserId(22)],
        }
    }

    /// Each excluded user is its own `exclude[]`.
    const ALL_PARAMS_QUERY: &str =
        "number=90&before=2026-11-01T00%3A00%3A00%2B00%3A00&exclude%5B%5D=11&exclude%5B%5D=22";

    #[rstest]
    fn post_likes(endpoint: ReaderLikesRequestEndpoint) {
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.post_likes(
                &WpComSiteId(3584907),
                &PostId(6014),
                &ReaderLikesParams::default(),
            ),
            "/sites/3584907/posts/6014/likes?",
        );
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.post_likes(&WpComSiteId(3584907), &PostId(6014), &all_params()),
            &format!("/sites/3584907/posts/6014/likes?{ALL_PARAMS_QUERY}"),
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> ReaderLikesRequestEndpoint {
        ReaderLikesRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
