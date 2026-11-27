use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace,
        reader_seen::{ReaderSeenFeedItemsParams, ReaderSeenResponse, ReaderSeenSitePostsParams},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

// Every request here needs an account.
#[derive(WpDerivedRequest)]
enum ReaderSeenRequest {
    // Marks posts of a WordPress.com or Jetpack site as seen.
    #[post(url = "/seen-posts/seen/blog/new", params = &ReaderSeenSitePostsParams, output = ReaderSeenResponse)]
    SitePostsSeen,
    // Marks posts of a WordPress.com or Jetpack site as unseen.
    #[post(url = "/seen-posts/seen/blog/delete", params = &ReaderSeenSitePostsParams, output = ReaderSeenResponse)]
    SitePostsUnseen,
    // Marks posts of an RSS feed as seen.
    #[post(url = "/seen-posts/seen/new", params = &ReaderSeenFeedItemsParams, output = ReaderSeenResponse)]
    FeedItemsSeen,
    // Marks posts of an RSS feed as unseen.
    #[post(url = "/seen-posts/seen/delete", params = &ReaderSeenFeedItemsParams, output = ReaderSeenResponse)]
    FeedItemsUnseen,
}

impl DerivedRequest for ReaderSeenRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::V2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::ApiUrlResolver,
        wp_com::endpoint::tests::{fixture_wp_com_api_url_resolver, validate_wp_com_v2_endpoint},
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn urls(endpoint: ReaderSeenRequestEndpoint) {
        validate_wp_com_v2_endpoint(endpoint.site_posts_seen(), "/seen-posts/seen/blog/new");
        validate_wp_com_v2_endpoint(endpoint.site_posts_unseen(), "/seen-posts/seen/blog/delete");
        validate_wp_com_v2_endpoint(endpoint.feed_items_seen(), "/seen-posts/seen/new");
        validate_wp_com_v2_endpoint(endpoint.feed_items_unseen(), "/seen-posts/seen/delete");
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> ReaderSeenRequestEndpoint {
        ReaderSeenRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
