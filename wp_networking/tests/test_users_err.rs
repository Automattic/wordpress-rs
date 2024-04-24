use wp_api::{
    parse_retrieve_user_response_with_edit_context, UserDeleteParams, UserId, WPContext,
    WPErrorCode,
};

use crate::test_helpers::{api, AssertWpError, WPNetworkRequestExecutor, FIRST_USER_ID};

pub mod test_helpers;
pub mod wp_db;

#[tokio::test]
async fn retrieve_user_invalid_user_id() {
    test_helpers::retrieve_user(UserId(987654321), WPContext::Edit, |p| {
        parse_retrieve_user_response_with_edit_context(&p)
    })
    .await
    .assert_wp_error(WPErrorCode::UserInvalidId, 404);
}

#[tokio::test]
async fn delete_user_invalid_reassign() {
    let user_delete_params = UserDeleteParams {
        reassign: UserId(987654321),
    };
    let response = api()
        .delete_user_request(FIRST_USER_ID, &user_delete_params)
        .execute()
        .await
        .unwrap();
    parse_retrieve_user_response_with_edit_context(&response)
        .assert_wp_error(WPErrorCode::UserInvalidReassign, 400);
}
