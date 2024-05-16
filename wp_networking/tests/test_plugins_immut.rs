use rstest::*;
use wp_api::{generate, plugins::PluginListParams, plugins::PluginStatus, WPContext};

use crate::test_helpers::{api, WPNetworkRequestExecutor};

pub mod test_helpers;
#[rstest]
#[case(PluginListParams::default())]
#[case(generate!(PluginListParams, (search, Some("foo".to_string()))))]
#[case(generate!(PluginListParams, (status, Some(PluginStatus::Active))))]
#[case(generate!(PluginListParams, (search, Some("foo".to_string()))))]
#[trace]
#[tokio::test]
async fn test_plugin_list_params_parametrized(
    #[case] params: PluginListParams,
    #[values(WPContext::Edit, WPContext::Embed, WPContext::View)] context: WPContext,
) {
    let response = api()
        .list_plugins_request(context, &Some(params))
        .execute()
        .await
        .unwrap();
    match context {
        WPContext::Edit => {
            let parsed_response = wp_api::parse_list_plugins_response_with_edit_context(&response);
            assert!(
                parsed_response.is_ok(),
                "Response was: '{:?}'",
                parsed_response
            );
        }
        WPContext::Embed => {
            let parsed_response = wp_api::parse_list_plugins_response_with_embed_context(&response);
            assert!(
                parsed_response.is_ok(),
                "Response was: '{:?}'",
                parsed_response
            );
        }
        WPContext::View => {
            let parsed_response = wp_api::parse_list_plugins_response_with_view_context(&response);
            assert!(
                parsed_response.is_ok(),
                "Response was: '{:?}'",
                parsed_response
            );
        }
    };
}
