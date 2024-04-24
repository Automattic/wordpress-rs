use wp_api::{WPApiHelper, WPAuthentication, WPContext, WPErrorCode};

use crate::test_helpers::{AssertWpError, WPNetworkRequestExecutor, WPNetworkResponseParser};

pub mod test_helpers;

#[tokio::test]
async fn unauthorized() {
    WPApiHelper::new(
        test_helpers::test_credentials().site_url,
        WPAuthentication::None,
    )
    .retrieve_current_user_request(WPContext::Edit)
    .execute()
    .await
    .unwrap()
    .parse(wp_api::parse_retrieve_user_response_with_edit_context)
    .assert_wp_error(WPErrorCode::Unauthorized);
}
