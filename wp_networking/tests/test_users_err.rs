use test_helpers::{wp_networking, FIRST_USER_ID};
use wp_api::{
    parse_retrieve_user_response_with_edit_context, ClientErrorType, UserDeleteParams, UserId,
    WPApiError, WPCodedError, WPContext, WPErrorCode,
};

pub mod test_helpers;
pub mod wp_db;

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
                    code: WPErrorCode::UserInvalidId
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

#[tokio::test]
async fn delete_user_invalid_reassign() {
    let user_delete_params = UserDeleteParams {
        reassign: UserId(987654321),
    };
    let user_delete_request = wp_networking()
        .api_helper
        .delete_user_request(FIRST_USER_ID, &user_delete_params);
    let err = parse_retrieve_user_response_with_edit_context(
        &wp_networking()
            .async_request(user_delete_request)
            .await
            .unwrap(),
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            WPApiError::ClientError {
                coded_error: Some(WPCodedError {
                    code: WPErrorCode::UserInvalidReassign
                }),
                error_type: ClientErrorType::BadRequest,
                status_code: 400,
                response: _
            }
        ),
        "{:?}",
        err
    );
}
