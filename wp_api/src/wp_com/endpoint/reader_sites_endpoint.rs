use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace,
        reader_saved::ReaderFeedId,
        reader_sites::{
            ReaderFeed, ReaderFeedSearchParams, ReaderFeedSearchResponse, ReaderSite,
            ReaderSiteParams,
        },
        sites::WpComSiteIdentifier,
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum ReaderSitesRequest {
    // A WordPress.com or Jetpack site's Reader info. The site can be an ID or a domain. Answers
    // signed out for a public site, with nothing about the user. A site that isn't there is a
    // 404 `unknown_blog`, a deleted one a 410 `site_gone`, and a private one the user can't read
    // a 403 `unauthorized`.
    #[get(url = "/read/sites/<wp_com_site_identifier>", params = &ReaderSiteParams, output = ReaderSite)]
    Site,
    // A feed's Reader info. Answers signed out for a public feed, with nothing about the user. A
    // feed that isn't there is a 404 `no_such_feed`, and the feed of a private site the user
    // can't read a 403 `unauthorized`.
    #[get(url = "/read/feed/<reader_feed_id>", output = ReaderFeed)]
    Feed,
    // Searches sites and feeds for some text, or finds the feeds at an address. Answers signed
    // out. With neither it's a 400 `missing_param`, and an address with no feed is a 404
    // `no_such_feed`.
    #[get(url = "/read/feed", params = &ReaderFeedSearchParams, output = ReaderFeedSearchResponse)]
    SearchFeeds,
}

impl DerivedRequest for ReaderSitesRequest {
    fn namespace(&self) -> impl AsNamespace {
        match self {
            // The newest version of each. `rest/v1.2` and `rest/v1.3` have no feed info or search.
            Self::Site => WpComNamespace::RestV1_2,
            Self::Feed | Self::SearchFeeds => WpComNamespace::RestV1_1,
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
                fixture_wp_com_api_url_resolver, validate_wp_com_rest_v1_1_endpoint,
                validate_wp_com_rest_v1_2_endpoint,
            },
            reader_sites::ReaderFeedSearchSort,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case::by_id(
        WpComSiteIdentifier::Id { value: 3584907 },
        ReaderSiteParams::default(),
        "/read/sites/3584907?"
    )]
    #[case::by_domain(
        WpComSiteIdentifier::Slug { value: "en.blog.wordpress.com".to_string() },
        ReaderSiteParams::default(),
        "/read/sites/en.blog.wordpress.com?"
    )]
    #[case::all_params(
        WpComSiteIdentifier::Id { value: 3584907 },
        ReaderSiteParams {
            fields: Some("ID,name,feed_ID".to_string()),
            options: Some("is_mapped_domain,unmapped_url".to_string()),
        },
        "/read/sites/3584907?fields=ID%2Cname%2Cfeed_ID&options=is_mapped_domain%2Cunmapped_url"
    )]
    fn site(
        endpoint: ReaderSitesRequestEndpoint,
        #[case] identifier: WpComSiteIdentifier,
        #[case] params: ReaderSiteParams,
        #[case] expected_path: &str,
    ) {
        validate_wp_com_rest_v1_2_endpoint(endpoint.site(&identifier, &params), expected_path);
    }

    #[rstest]
    fn feed(endpoint: ReaderSitesRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.feed(&ReaderFeedId(8857199)),
            "/read/feed/8857199",
        );
    }

    #[rstest]
    #[case::default(ReaderFeedSearchParams::default(), "/read/feed?")]
    #[case::by_address(
        ReaderFeedSearchParams {
            url: Some("https://example.com/feed/".to_string()),
            ..Default::default()
        },
        "/read/feed?url=https%3A%2F%2Fexample.com%2Ffeed%2F"
    )]
    #[case::all_params(
        ReaderFeedSearchParams {
            q: Some("photo graphy".to_string()),
            url: Some("example.com".to_string()),
            number: Some(20),
            offset: Some(40),
            exclude_followed: Some(false),
            sort: Some(ReaderFeedSearchSort::LastUpdated),
            page_handle: Some("last=1&blog=2&feed=0".to_string()),
            algorithm: Some("a".to_string()),
            meta: Some("site,feed".to_string()),
        },
        "/read/feed?q=photo+graphy&url=example.com&number=20&offset=40&exclude_followed=false&sort=last_updated&page_handle=last%3D1%26blog%3D2%26feed%3D0&algorithm=a&meta=site%2Cfeed"
    )]
    fn search_feeds(
        endpoint: ReaderSitesRequestEndpoint,
        #[case] params: ReaderFeedSearchParams,
        #[case] expected_path: &str,
    ) {
        validate_wp_com_rest_v1_1_endpoint(endpoint.search_feeds(&params), expected_path);
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> ReaderSitesRequestEndpoint {
        ReaderSitesRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
