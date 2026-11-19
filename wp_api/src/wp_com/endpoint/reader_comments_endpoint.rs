use crate::{
    posts::PostId,
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        reader_comments::{
            ReaderCommentsParams, ReaderCommentsResponse, ReaderPostCommentsSubscription,
        },
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum ReaderCommentsRequest {
    // A post's comments, by offset, page number or date. Answers signed out for a public site; a
    // private site's is then a 403 `unauthorized`. A post that doesn't exist is a 404
    // `unknown_post`.
    #[get(url = "/sites/<wp_com_site_id>/posts/<post_id>/replies", params = &ReaderCommentsParams, output = ReaderCommentsResponse)]
    List,
    // Whether the user is subscribed to a post's comments. Needs a token: signed out it's a 403
    // `authorization_required`.
    #[get(url = "/sites/<wp_com_site_id>/posts/<post_id>/subscribers/mine", output = ReaderPostCommentsSubscription)]
    Subscription,
}

impl DerivedRequest for ReaderCommentsRequest {
    fn namespace(&self) -> impl AsNamespace {
        // The only version of both: `rest/v1.2` has neither.
        WpComNamespace::RestV1_1
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
            },
            reader_comments::{
                ReaderCommentsOrder, ReaderCommentsStatusFilter, ReaderCommentsTypeFilter,
            },
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case::default(ReaderCommentsParams::default(), "/sites/3584907/posts/6014/replies?")]
    #[case::all_params(
        ReaderCommentsParams {
            number: Some(100),
            offset: Some(200),
            page: Some(3),
            order: Some(ReaderCommentsOrder::Asc),
            hierarchical: Some(true),
            after: Some("2026-10-01T00:00:00Z".parse().unwrap()),
            before: Some("2026-11-01T00:00:00Z".parse().unwrap()),
            comment_type: Some(ReaderCommentsTypeFilter::Pings),
            status: Some(ReaderCommentsStatusFilter::Approved),
            author_wpcom_data: Some(true),
            force: Some("wpcom".to_string()),
            fields: Some("ID,date,parent".to_string()),
        },
        "/sites/3584907/posts/6014/replies?number=100&offset=200&page=3&order=ASC&hierarchical=true&after=2026-10-01T00%3A00%3A00%2B00%3A00&before=2026-11-01T00%3A00%3A00%2B00%3A00&type=pings&status=approved&author_wpcom_data=true&force=wpcom&fields=ID%2Cdate%2Cparent"
    )]
    fn list(
        endpoint: ReaderCommentsRequestEndpoint,
        #[case] params: ReaderCommentsParams,
        #[case] expected_path: &str,
    ) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.list(&WpComSiteId(3584907), &PostId(6014), &params),
            expected_path,
        );
    }

    #[rstest]
    fn subscription(endpoint: ReaderCommentsRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.subscription(&WpComSiteId(3584907), &PostId(6014)),
            "/sites/3584907/posts/6014/subscribers/mine",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> ReaderCommentsRequestEndpoint {
        ReaderCommentsRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
