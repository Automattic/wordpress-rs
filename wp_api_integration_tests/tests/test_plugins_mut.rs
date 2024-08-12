use rstest::rstest;
use serial_test::serial;
use wp_api::plugins::{PluginCreateParams, PluginSlug, PluginStatus, PluginUpdateParams};
use wp_api_integration_tests::{
    api_client, AssertResponse, CLASSIC_EDITOR_PLUGIN_SLUG, HELLO_DOLLY_PLUGIN_SLUG,
    WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS,
};
use wp_api_integration_tests::{BackendSupport, ServerRestore};

#[tokio::test]
#[serial]
async fn create_plugin() {
    let status = PluginStatus::Active;
    let params = PluginCreateParams {
        slug: WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS.into(),
        status,
    };
    let created_plugin = api_client()
        .plugins()
        .create(&params)
        .await
        .assert_response();
    assert_eq!(created_plugin.status, status);
    println!("Created Plugin: {:?}", created_plugin);

    BackendSupport::restore(ServerRestore::all()).await;
}

#[rstest]
#[case(PluginSlug::new(HELLO_DOLLY_PLUGIN_SLUG.into()), PluginStatus::Active)]
#[case(PluginSlug::new(CLASSIC_EDITOR_PLUGIN_SLUG.into()), PluginStatus::Inactive)]
#[trace]
#[tokio::test]
#[serial]
async fn update_plugin(#[case] slug: PluginSlug, #[case] new_status: PluginStatus) {
    let updated_plugin = api_client()
        .plugins()
        .update(&slug, &PluginUpdateParams { status: new_status })
        .await
        .assert_response();
    assert_eq!(updated_plugin.status, new_status);
    println!("Updated Plugin: {:?}", updated_plugin);

    BackendSupport::restore(ServerRestore::all()).await;
}

#[tokio::test]
#[serial]
async fn delete_plugin() {
    let slug = CLASSIC_EDITOR_PLUGIN_SLUG.into();
    let deleted_plugin = api_client().plugins().delete(&slug).await.assert_response();
    assert_eq!(slug, deleted_plugin.previous.plugin);
    println!("Deleted Plugin: {:?}", deleted_plugin);

    BackendSupport::restore(ServerRestore::all()).await;
}
