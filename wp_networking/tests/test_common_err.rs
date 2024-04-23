use test_helpers::wp_networking;
use wp_api::{
    parse_retrieve_user_response_with_edit_context, ClientErrorType, WPApiError, WPAuthentication,
    WPContext,
};
use wp_networking::AsyncWPNetworking;

pub mod test_helpers;

#[tokio::test]
async fn client_error_unauthorized() {
    let (site_url, _, _) = test_helpers::test_credentials();
    let n = AsyncWPNetworking::new(site_url.into(), WPAuthentication::None);
    let request = n.api_helper.retrieve_current_user_request(WPContext::Edit);
    let response = wp_networking().async_request(request).await.unwrap();
    let err = parse_retrieve_user_response_with_edit_context(&response).unwrap_err();
    assert!(
        matches!(
            err,
            WPApiError::ClientError {
                coded_error: _,
                error_type: ClientErrorType::Unauthorized,
                status_code: 401,
                response: _
            }
        ),
        "{:?}",
        err
    );
}
