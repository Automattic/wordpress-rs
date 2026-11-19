use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        reader_posts::{
            ReaderFreshlyPressedParams, ReaderLikedPostsParams, ReaderOrganizationPostsParams,
            ReaderPostsResponse, ReaderStreamParams, ReaderTagPostsParams, ReaderTagPostsResponse,
            ReaderTagSlug,
        },
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
    // A tag's posts.
    #[get(url = "/read/tags/<reader_tag_slug>/posts", params = &ReaderTagPostsParams, output = ReaderTagPostsResponse)]
    TagPosts,
    // The posts the user liked, newest like first. Signed out, the API answers 403.
    #[get(url = "/read/liked", params = &ReaderLikedPostsParams, output = ReaderPostsResponse)]
    Liked,
    // Posts from Automattic's sites, for Automatticians. Signed out, the API answers with a page
    // with no posts.
    #[get(url = "/read/a8c", params = &ReaderOrganizationPostsParams, output = ReaderPostsResponse)]
    Automattic,
    // Posts from the P2s the user follows. Signed out, the API answers 403.
    #[get(url = "/read/following/p2", params = &ReaderOrganizationPostsParams, output = ReaderPostsResponse)]
    FollowingP2,
    // Freshly Pressed: the posts WordPress.com's editors picked. Answers signed out.
    #[get(url = "/freshly-pressed", params = &ReaderFreshlyPressedParams, output = ReaderPostsResponse)]
    FreshlyPressed,
}

impl DerivedRequest for ReaderPostsRequest {
    fn namespace(&self) -> impl AsNamespace {
        match self {
            // The newest tag stream. `rest/v1.2` and `rest/v1.3` have older ones.
            Self::TagPosts => WpComNamespace::V2,
            // `rest/v1.2` is the newest Automattic stream with whole posts: `rest/v1.3` returns
            // only each post's IDs and date.
            _ => WpComNamespace::RestV1_2,
        }
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
                validate_wp_com_v2_endpoint,
            },
            reader_posts::{ReaderStreamOrder, ReaderTagPostsSort},
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
            fields: Some("ID,site_ID,date".to_string()),
        }
    }

    const ALL_PARAMS_QUERY: &str = "number=7&before=2026-11-01T00%3A00%3A00%2B00%3A00&after=2026-10-01T00%3A00%3A00%2B00%3A00&meta=site%2Clikes&fields=ID%2Csite_ID%2Cdate";

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

    #[rstest]
    fn tag_posts(endpoint: ReaderPostsRequestEndpoint) {
        validate_wp_com_v2_endpoint(
            endpoint.tag_posts(
                &ReaderTagSlug("photography".to_string()),
                &ReaderTagPostsParams::default(),
            ),
            "/read/tags/photography/posts?",
        );
        validate_wp_com_v2_endpoint(
            endpoint.tag_posts(
                &ReaderTagSlug("photography".to_string()),
                &ReaderTagPostsParams {
                    number: Some(10),
                    page_handle: Some("abc==".to_string()),
                    sort: Some(ReaderTagPostsSort::Popularity),
                    before: Some("2026-11-01T00:00:00Z".parse().unwrap()),
                    after: Some("2026-10-01T00:00:00Z".parse().unwrap()),
                },
            ),
            "/read/tags/photography/posts?number=10&page_handle=abc%3D%3D&sort=popularity&before=2026-11-01T00%3A00%3A00%2B00%3A00&after=2026-10-01T00%3A00%3A00%2B00%3A00",
        );
    }

    /// A tag's slug can hold any character, such as Japanese, so it's escaped in the path.
    #[rstest]
    fn tag_posts_escapes_the_slug(endpoint: ReaderPostsRequestEndpoint) {
        validate_wp_com_v2_endpoint(
            endpoint.tag_posts(
                &ReaderTagSlug("写真".to_string()),
                &ReaderTagPostsParams::default(),
            ),
            "/read/tags/%E5%86%99%E7%9C%9F/posts?",
        );
    }

    #[rstest]
    fn liked(endpoint: ReaderPostsRequestEndpoint) {
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.liked(&ReaderLikedPostsParams::default()),
            "/read/liked?",
        );
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.liked(&ReaderLikedPostsParams {
                number: Some(40),
                before: Some("2026-11-01T00:00:00Z".parse().unwrap()),
                after: Some("2026-10-01T00:00:00Z".parse().unwrap()),
                order: Some(ReaderStreamOrder::Asc),
                page: Some(2),
                meta: Some("site,likes".to_string()),
                fields: Some("ID,site_ID,date_liked".to_string()),
            }),
            "/read/liked?number=40&before=2026-11-01T00%3A00%3A00%2B00%3A00&after=2026-10-01T00%3A00%3A00%2B00%3A00&order=ASC&page=2&meta=site%2Clikes&fields=ID%2Csite_ID%2Cdate_liked",
        );
    }

    fn all_organization_params() -> ReaderOrganizationPostsParams {
        ReaderOrganizationPostsParams {
            number: Some(40),
            before: Some("2026-11-01T00:00:00Z".parse().unwrap()),
            after: Some("2026-10-01T00:00:00Z".parse().unwrap()),
            order: Some(ReaderStreamOrder::Desc),
            page: Some(2),
            meta: Some("site,likes".to_string()),
            fields: Some("ID,site_ID,date".to_string()),
        }
    }

    const ALL_ORGANIZATION_PARAMS_QUERY: &str = "number=40&before=2026-11-01T00%3A00%3A00%2B00%3A00&after=2026-10-01T00%3A00%3A00%2B00%3A00&order=DESC&page=2&meta=site%2Clikes&fields=ID%2Csite_ID%2Cdate";

    #[rstest]
    fn automattic(endpoint: ReaderPostsRequestEndpoint) {
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.automattic(&ReaderOrganizationPostsParams::default()),
            "/read/a8c?",
        );
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.automattic(&all_organization_params()),
            &format!("/read/a8c?{ALL_ORGANIZATION_PARAMS_QUERY}"),
        );
    }

    #[rstest]
    fn following_p_2(endpoint: ReaderPostsRequestEndpoint) {
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.following_p_2(&ReaderOrganizationPostsParams::default()),
            "/read/following/p2?",
        );
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.following_p_2(&all_organization_params()),
            &format!("/read/following/p2?{ALL_ORGANIZATION_PARAMS_QUERY}"),
        );
    }

    #[rstest]
    fn freshly_pressed(endpoint: ReaderPostsRequestEndpoint) {
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.freshly_pressed(&ReaderFreshlyPressedParams::default()),
            "/freshly-pressed?",
        );
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.freshly_pressed(&ReaderFreshlyPressedParams {
                number: Some(40),
                before: Some("2026-11-01T00:00:00Z".parse().unwrap()),
                after: Some("2026-10-01T00:00:00Z".parse().unwrap()),
                content_width: Some(675),
                thumb_width: Some(252),
                thumb_height: Some(160),
                meta: Some("site,likes".to_string()),
                fields: Some("ID,site_ID,date,editorial".to_string()),
            }),
            "/freshly-pressed?number=40&before=2026-11-01T00%3A00%3A00%2B00%3A00&after=2026-10-01T00%3A00%3A00%2B00%3A00&content_width=675&thumb_width=252&thumb_height=160&meta=site%2Clikes&fields=ID%2Csite_ID%2Cdate%2Ceditorial",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> ReaderPostsRequestEndpoint {
        ReaderPostsRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
