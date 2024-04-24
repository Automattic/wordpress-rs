use wp_api::{UserDeleteParams, UserId, WPContext, WPErrorCode};

use crate::test_helpers::{
    api, AssertWpError, WPNetworkRequestExecutor, WPNetworkResponseParser, FIRST_USER_ID,
};

pub mod test_helpers;
pub mod wp_db;

#[tokio::test]
async fn retrieve_user_invalid_user_id() {
    api()
        .retrieve_user_request(UserId(987654321), WPContext::Edit)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPErrorCode::UserInvalidId, 404);
}

#[tokio::test]
async fn delete_user_invalid_reassign() {
    api()
        .delete_user_request(
            FIRST_USER_ID,
            &UserDeleteParams {
                reassign: UserId(987654321),
            },
        )
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_user_response_with_edit_context)
        .assert_wp_error(WPErrorCode::UserInvalidReassign, 400);
}
