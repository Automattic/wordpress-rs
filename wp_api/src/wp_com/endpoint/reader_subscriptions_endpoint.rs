use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        reader_subscriptions::{
            ReaderSiteFollowStatus, ReaderSubscriptionsParams, ReaderSubscriptionsResponse,
        },
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum ReaderSubscriptionsRequest {
    // The sites and feeds the user is subscribed to, by page number. Needs a token.
    #[get(url = "/read/following/mine", params = &ReaderSubscriptionsParams, output = ReaderSubscriptionsResponse)]
    List,
    // Whether the user follows a site. Needs a token. It takes a site's ID, not a feed's: a feed's
    // ID is read as some other site's.
    #[get(url = "/sites/<wp_com_site_id>/follows/mine", output = ReaderSiteFollowStatus)]
    IsFollowingSite,
}

impl DerivedRequest for ReaderSubscriptionsRequest {
    fn namespace(&self) -> impl AsNamespace {
        match self {
            // The newest list. `rest/v1.1` returns every subscription at once, without paging.
            Self::List => WpComNamespace::RestV1_2,
            // The only version: `rest/v1.2` has none.
            Self::IsFollowingSite => WpComNamespace::RestV1_1,
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
            validate_wp_com_rest_v1_2_endpoint,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case::default(ReaderSubscriptionsParams::default(), "/read/following/mine?")]
    #[case::all_params(
        ReaderSubscriptionsParams {
            page: Some(2),
            number: Some(100),
            q: Some("example blog".to_string()),
            filter: Some("a8c".to_string()),
            meta: Some("site,feed".to_string()),
        },
        "/read/following/mine?page=2&number=100&q=example+blog&filter=a8c&meta=site%2Cfeed"
    )]
    fn list(
        endpoint: ReaderSubscriptionsRequestEndpoint,
        #[case] params: ReaderSubscriptionsParams,
        #[case] expected_path: &str,
    ) {
        validate_wp_com_rest_v1_2_endpoint(endpoint.list(&params), expected_path);
    }

    #[rstest]
    fn is_following_site(endpoint: ReaderSubscriptionsRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.is_following_site(&WpComSiteId(3584907)),
            "/sites/3584907/follows/mine",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> ReaderSubscriptionsRequestEndpoint {
        ReaderSubscriptionsRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
