use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace,
        reader_saved::{
            ReaderSavePostParams, ReaderSavedBulkParams, ReaderSavedBulkResponse,
            ReaderSavedKeysResponse, ReaderSavedPostResponse, ReaderSavedPostsParams,
            ReaderSavedPostsResponse, ReaderUnsavePostParams,
        },
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum ReaderSavedRequest {
    #[get(url = "/read/saved", params = &ReaderSavedPostsParams, output = ReaderSavedPostsResponse)]
    ListPosts,
    #[get(url = "/read/saved/keys", output = ReaderSavedKeysResponse)]
    ListKeys,
    #[post(url = "/read/saved/new", params = &ReaderSavePostParams, output = ReaderSavedPostResponse)]
    Save,
    #[post(url = "/read/saved/delete", params = &ReaderUnsavePostParams, output = ReaderSavedPostResponse)]
    Unsave,
    #[post(url = "/read/saved/bulk", params = &ReaderSavedBulkParams, output = ReaderSavedBulkResponse)]
    BulkSave,
}

impl DerivedRequest for ReaderSavedRequest {
    fn namespace(&self) -> impl AsNamespace {
        match self {
            Self::ListPosts => WpComNamespace::RestV1_2,
            _ => WpComNamespace::V2,
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
            validate_wp_com_v2_endpoint,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case::default(ReaderSavedPostsParams::default(), "/read/saved?")]
    #[case::all_params(
        ReaderSavedPostsParams {
            page_handle: Some("abc==".to_string()),
            number: Some(20),
            after: Some("2026-10-01T00:00:00Z".parse().unwrap()),
            before: Some("2026-11-01T00:00:00Z".parse().unwrap()),
        },
        "/read/saved?page_handle=abc%3D%3D&number=20&after=2026-10-01T00%3A00%3A00%2B00%3A00&before=2026-11-01T00%3A00%3A00%2B00%3A00"
    )]
    fn list_posts(
        endpoint: ReaderSavedRequestEndpoint,
        #[case] params: ReaderSavedPostsParams,
        #[case] expected_path: &str,
    ) {
        validate_wp_com_rest_v1_2_endpoint(endpoint.list_posts(&params), expected_path);
    }

    #[rstest]
    fn list_keys(endpoint: ReaderSavedRequestEndpoint) {
        validate_wp_com_v2_endpoint(endpoint.list_keys(), "/read/saved/keys");
    }

    #[rstest]
    fn save(endpoint: ReaderSavedRequestEndpoint) {
        validate_wp_com_v2_endpoint(endpoint.save(), "/read/saved/new");
    }

    #[rstest]
    fn unsave(endpoint: ReaderSavedRequestEndpoint) {
        validate_wp_com_v2_endpoint(endpoint.unsave(), "/read/saved/delete");
    }

    #[rstest]
    fn bulk_save(endpoint: ReaderSavedRequestEndpoint) {
        validate_wp_com_v2_endpoint(endpoint.bulk_save(), "/read/saved/bulk");
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> ReaderSavedRequestEndpoint {
        ReaderSavedRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
