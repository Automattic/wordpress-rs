use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        reader_posts::{ReaderPostsResponse, ReaderStreamParams},
        reader_saved::ReaderFeedId,
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum ReaderPostsRequest {
    // Posts from the sites and feeds the user follows.
    #[get(url = "/read/following", params = &ReaderStreamParams, output = ReaderPostsResponse)]
    Following,
    // A WordPress.com site's posts.
    #[get(url = "/read/sites/<wp_com_site_id>/posts", params = &ReaderStreamParams, output = ReaderPostsResponse)]
    SitePosts,
    // An RSS feed's posts.
    #[get(url = "/read/feed/<reader_feed_id>/posts", params = &ReaderStreamParams, output = ReaderPostsResponse)]
    FeedPosts,
}

impl DerivedRequest for ReaderPostsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::ApiUrlResolver,
        wp_com::endpoint::tests::{
            fixture_wp_com_api_url_resolver, validate_wp_com_rest_v1_2_endpoint,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    fn all_params() -> ReaderStreamParams {
        ReaderStreamParams {
            number: Some(7),
            before: Some("2026-11-01T00:00:00Z".parse().unwrap()),
            after: Some("2026-10-01T00:00:00Z".parse().unwrap()),
            meta: Some("site,likes".to_string()),
        }
    }

    const ALL_PARAMS_QUERY: &str = "number=7&before=2026-11-01T00%3A00%3A00%2B00%3A00&after=2026-10-01T00%3A00%3A00%2B00%3A00&meta=site%2Clikes";

    #[rstest]
    fn following(endpoint: ReaderPostsRequestEndpoint) {
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.following(&ReaderStreamParams::default()),
            "/read/following?",
        );
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.following(&all_params()),
            &format!("/read/following?{ALL_PARAMS_QUERY}"),
        );
    }

    #[rstest]
    fn site_posts(endpoint: ReaderPostsRequestEndpoint) {
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.site_posts(&WpComSiteId(3584907), &ReaderStreamParams::default()),
            "/read/sites/3584907/posts?",
        );
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.site_posts(&WpComSiteId(3584907), &all_params()),
            &format!("/read/sites/3584907/posts?{ALL_PARAMS_QUERY}"),
        );
    }

    #[rstest]
    fn feed_posts(endpoint: ReaderPostsRequestEndpoint) {
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.feed_posts(&ReaderFeedId(8857199), &ReaderStreamParams::default()),
            "/read/feed/8857199/posts?",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> ReaderPostsRequestEndpoint {
        ReaderPostsRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
