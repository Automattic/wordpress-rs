use crate::{
    posts::PostId,
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        reader_post::{
            ReaderPostBySlug, ReaderPostBySlugParams, ReaderPostParams, ReaderPostSlug,
            ReaderRelatedPostsParams, ReaderRelatedPostsResponse,
        },
        reader_posts::ReaderPost,
        reader_saved::{ReaderFeedId, ReaderFeedItemId},
        sites::WpComSiteIdentifier,
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum ReaderPostRequest {
    // One post of a WordPress.com or Jetpack site, in the shape a stream sends. Answers signed
    // out for a public site. A post that isn't there is a 404 `unknown_post`, and a site that
    // isn't a 404 `unknown_blog`. A private site's post is a 403 `unauthorized` to anyone who
    // can't read the site, and so is a trashed post asked for signed out.
    //
    // For a trashed post asked for by someone who can edit it, and for a post of a deleted site,
    // the API answers with an HTTP 400 whose `error` is the number 404. That isn't an error code,
    // so it arrives as `WpApiError::UnknownError`.
    #[get(url = "/read/sites/<wp_com_site_id>/posts/<post_id>", params = &ReaderPostParams, output = ReaderPost)]
    SitePost,
    // One post of a feed, in the shape a stream sends. Answers signed out. An item that isn't in
    // the feed is a 404 `no_such_item`, and a feed that isn't there a 404 `no_such_feed`.
    //
    // A stream can list an item this request can't fetch. The Following stream sends some posts
    // of a Jetpack site as external feed items, and several of those are a 404 `post_not_found`
    // here.
    #[get(url = "/read/feed/<reader_feed_id>/posts/<reader_feed_item_id>", params = &ReaderPostParams, output = ReaderPost)]
    FeedPost,
    // A site's post, found by its slug. The site can be an ID or a domain. This is the general
    // posts endpoint, so the post isn't in the shape a stream sends. Answers signed out for a
    // public site. A slug that isn't there is a 404 `unknown_post`, and a private site's post a
    // 403 `unauthorized` to anyone who can't read the site.
    #[get(url = "/sites/<wp_com_site_identifier>/posts/<reader_post_slug>", params = &ReaderPostBySlugParams, output = ReaderPostBySlug)]
    PostBySlug,
    // The posts related to a site's post. Answers signed out for a public site. A post that isn't
    // there is a 404 `unknown_post`, and a private site's post a 403 `unauthorized` to anyone
    // who can't read the site.
    #[get(url = "/read/site/<wp_com_site_id>/post/<post_id>/related", params = &ReaderRelatedPostsParams, output = ReaderRelatedPostsResponse)]
    RelatedPosts,
}

impl DerivedRequest for ReaderPostRequest {
    fn namespace(&self) -> impl AsNamespace {
        match self {
            // The newest version of each. `rest/v1.3` has no site post or related posts, and the
            // general posts endpoint stops at `rest/v1.1`.
            Self::SitePost | Self::RelatedPosts => WpComNamespace::RestV1_2,
            Self::FeedPost => WpComNamespace::RestV1_3,
            Self::PostBySlug => WpComNamespace::RestV1_1,
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
        },
    };
    use rstest::*;
    use std::sync::Arc;

    const SITE: WpComSiteId = WpComSiteId(3584907);

    fn all_post_params() -> ReaderPostParams {
        ReaderPostParams {
            meta: Some("site,likes".to_string()),
            fields: Some("ID,site_ID,date".to_string()),
        }
    }

    const ALL_POST_PARAMS_QUERY: &str = "meta=site%2Clikes&fields=ID%2Csite_ID%2Cdate";

    #[rstest]
    fn site_post(endpoint: ReaderPostRequestEndpoint) {
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.site_post(&SITE, &PostId(29614), &ReaderPostParams::default()),
            "/read/sites/3584907/posts/29614?",
        );
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.site_post(&SITE, &PostId(29614), &all_post_params()),
            &format!("/read/sites/3584907/posts/29614?{ALL_POST_PARAMS_QUERY}"),
        );
    }

    #[rstest]
    fn feed_post(endpoint: ReaderPostRequestEndpoint) {
        validate_wp_com_rest_v1_3_endpoint(
            endpoint.feed_post(
                &ReaderFeedId(8857199),
                &ReaderFeedItemId(6250120687),
                &ReaderPostParams::default(),
            ),
            "/read/feed/8857199/posts/6250120687?",
        );
        validate_wp_com_rest_v1_3_endpoint(
            endpoint.feed_post(
                &ReaderFeedId(8857199),
                &ReaderFeedItemId(6250120687),
                &all_post_params(),
            ),
            &format!("/read/feed/8857199/posts/6250120687?{ALL_POST_PARAMS_QUERY}"),
        );
    }

    #[rstest]
    #[case::by_site_id(
        WpComSiteIdentifier::Id { value: 3584907 },
        ReaderPostBySlugParams::default(),
        "/sites/3584907/posts/slug:hello-world?"
    )]
    #[case::by_domain(
        WpComSiteIdentifier::Slug { value: "en.blog.wordpress.com".to_string() },
        ReaderPostBySlugParams { fields: Some("ID,site_ID".to_string()) },
        "/sites/en.blog.wordpress.com/posts/slug:hello-world?fields=ID%2Csite_ID"
    )]
    fn post_by_slug(
        endpoint: ReaderPostRequestEndpoint,
        #[case] site: WpComSiteIdentifier,
        #[case] params: ReaderPostBySlugParams,
        #[case] expected_path: &str,
    ) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.post_by_slug(&site, &ReaderPostSlug("hello-world".to_string()), &params),
            expected_path,
        );
    }

    /// A slug can hold any character, such as Japanese, so it's escaped in the path.
    #[rstest]
    fn post_by_slug_escapes_the_slug(endpoint: ReaderPostRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.post_by_slug(
                &WpComSiteIdentifier::Id { value: 3584907 },
                &ReaderPostSlug("写真".to_string()),
                &ReaderPostBySlugParams::default(),
            ),
            "/sites/3584907/posts/slug:%E5%86%99%E7%9C%9F?",
        );
    }

    #[rstest]
    fn related_posts(endpoint: ReaderPostRequestEndpoint) {
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.related_posts(&SITE, &PostId(29614), &ReaderRelatedPostsParams::default()),
            "/read/site/3584907/post/29614/related?",
        );
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.related_posts(
                &SITE,
                &PostId(29614),
                &ReaderRelatedPostsParams {
                    size_global: Some(2),
                    size_local: Some(3),
                    lang: Some("en".to_string()),
                    algorithm: Some("a".to_string()),
                    algorithm_local: Some("b".to_string()),
                    algorithm_global: Some("c".to_string()),
                    shuffle_posts_local: Some(true),
                    shuffle_posts_global: Some(false),
                    meta: Some("site".to_string()),
                    fields: Some("ID,site_ID,railcar".to_string()),
                },
            ),
            "/read/site/3584907/post/29614/related?size_global=2&size_local=3&lang=en&algorithm=a&algorithm_local=b&algorithm_global=c&shuffle_posts_local=true&shuffle_posts_global=false&meta=site&fields=ID%2Csite_ID%2Crailcar",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> ReaderPostRequestEndpoint {
        ReaderPostRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
