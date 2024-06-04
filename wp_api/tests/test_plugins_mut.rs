use rstest::rstest;
use wp_api::plugins::{PluginCreateParams, PluginSlug, PluginStatus, PluginUpdateParams};

use crate::integration_test_common::{
    request_builder, run_and_restore_wp_content_plugins, CLASSIC_EDITOR_PLUGIN_SLUG,
    HELLO_DOLLY_PLUGIN_SLUG, WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS,
};

pub mod integration_test_common;
pub mod wp_db;

#[tokio::test]
async fn create_plugin() {
    run_and_restore_wp_content_plugins(|| {
        wp_db::run_and_restore(|mut _db| async move {
            let status = PluginStatus::Active;
            let params = PluginCreateParams {
                slug: WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS.into(),
                status,
            };
            let created_plugin = request_builder().plugins().create(&params).await.unwrap();
            assert_eq!(created_plugin.status, status);
            println!("Created Plugin: {:?}", created_plugin);
        })
    })
    .await;
}

#[rstest]
#[case(PluginSlug::new(HELLO_DOLLY_PLUGIN_SLUG.into()), PluginStatus::Active)]
#[case(PluginSlug::new(CLASSIC_EDITOR_PLUGIN_SLUG.into()), PluginStatus::Inactive)]
#[trace]
#[tokio::test]
async fn update_plugin(#[case] slug: PluginSlug, #[case] new_status: PluginStatus) {
    run_and_restore_wp_content_plugins(|| {
        wp_db::run_and_restore(|mut _db| async move {
            let updated_plugin = request_builder()
                .plugins()
                .update(&slug, &PluginUpdateParams { status: new_status })
                .await
                .unwrap();
            assert_eq!(updated_plugin.status, new_status);
            println!("Updated Plugin: {:?}", updated_plugin);
        })
    })
    .await;
}

#[tokio::test]
async fn delete_plugin() {
    run_and_restore_wp_content_plugins(|| {
        wp_db::run_and_restore(|mut _db| async move {
            let slug = CLASSIC_EDITOR_PLUGIN_SLUG.into();
            let deleted_plugin = request_builder().plugins().delete(&slug).await.unwrap();
            assert_eq!(slug, deleted_plugin.previous.plugin);
            println!("Deleted Plugin: {:?}", deleted_plugin);
        })
    })
    .await;
}
