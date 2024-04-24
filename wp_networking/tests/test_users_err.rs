use test_helpers::{wp_networking, AssertWpError, FIRST_USER_ID};
use wp_api::{
    parse_retrieve_user_response_with_edit_context, UserDeleteParams, UserId, WPContext,
    WPErrorCode,
};

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
    let user_delete_request = wp_networking()
        .api_helper
        .delete_user_request(FIRST_USER_ID, &user_delete_params);
    parse_retrieve_user_response_with_edit_context(
        &wp_networking()
            .async_request(user_delete_request)
            .await
            .unwrap(),
    )
    .assert_wp_error(WPErrorCode::UserInvalidReassign, 400);
}
