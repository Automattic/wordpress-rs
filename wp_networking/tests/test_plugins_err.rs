use wp_api::{WPContext, WPRestErrorCode};

use crate::test_helpers::{
    api_as_subscriber, AssertWpError, WPNetworkRequestExecutor, WPNetworkResponseParser,
};

pub mod test_helpers;

#[tokio::test]
async fn list_plugins_err_cannot_view_plugins() {
    api_as_subscriber()
        .list_plugins_request(WPContext::Edit, &None)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_plugin_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::CannotViewPlugins);
}
