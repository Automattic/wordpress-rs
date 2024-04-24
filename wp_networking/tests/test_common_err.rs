use wp_api::{
    parse_retrieve_user_response_with_edit_context, ClientErrorType, WPApiError, WPApiHelper,
    WPAuthentication, WPCodedError, WPContext, WPErrorCode,
};

use crate::test_helpers::WPNetworkRequestExecutor;

pub mod test_helpers;

#[tokio::test]
async fn client_error_unauthorized() {
    let (site_url, _, _) = test_helpers::test_credentials();
    let response = WPApiHelper::new(site_url.into(), WPAuthentication::None)
        .retrieve_current_user_request(WPContext::Edit)
        .execute()
        .await
        .unwrap();
    let err = parse_retrieve_user_response_with_edit_context(&response).unwrap_err();
    assert!(
        matches!(
            err,
            WPApiError::ClientError {
                coded_error: Some(WPCodedError {
                    code: WPErrorCode::Unauthorized
                }),
                error_type: ClientErrorType::Other,
                status_code: 401,
                response: _
            }
        ),
        "{:?}",
        err
    );
}
