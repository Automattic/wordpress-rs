use wp_api::{
    parse_retrieve_user_response_with_edit_context, ClientErrorType, UserId, WPApiError,
    WPCodedError, WPContext, WPErrorCode,
};

pub mod test_helpers;

#[tokio::test]
async fn retrieve_user_invalid_user_id() {
    let err = test_helpers::retrieve_user(UserId(987654321), WPContext::Edit, |p| {
        parse_retrieve_user_response_with_edit_context(&p)
    })
    .await
    .unwrap_err();
    assert!(
        matches!(
            err,
            WPApiError::ClientError {
                coded_error: Some(WPCodedError {
                    code: WPErrorCode::InvalidUserId
                }),
                error_type: ClientErrorType::Other,
                status_code: 404,
                response: _
            }
        ),
        "{:?}",
        err
    );
}
