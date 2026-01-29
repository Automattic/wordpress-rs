use wp_api::plugins::{
    PluginCreateParams, PluginStatus, PluginUpdateParams, PluginWpOrgDirectorySlug,
};
use wp_mobile_integration_tests::*;

#[tokio::test]
#[serial]
// PostTypeCollectionWithEditContext should not return custom post types
// after their plugin is deleted.
async fn test_plugin_post_type_registration_and_removal() {
    let ctx = create_test_context();

    ctx.api
        .plugins()
        .create(&PluginCreateParams {
            slug: PluginWpOrgDirectorySlug::from("newspack-newsletters"),
            status: PluginStatus::Active,
        })
        .await
        .expect("Failed to install Newspack Newsletters plugin");

    let collection = ctx
        .service
        .post_types()
        .create_post_type_collection_with_edit_context();

    collection
        .fetch()
        .await
        .expect("Failed to fetch post types after plugin installation");

    let post_types = collection
        .load_data()
        .await
        .expect("Failed to load post types from cache");

    assert!(
        post_types
            .iter()
            .any(|pt| pt.data.slug == "newspack_nl_cpt"),
        "Newspack Newsletters custom post type should be present after plugin installation"
    );

    let slug =
        wp_api::plugins::PluginSlug::new("newspack-newsletters/newspack-newsletters".to_string());
    ctx.api
        .plugins()
        .update(
            &slug,
            &PluginUpdateParams {
                status: PluginStatus::Inactive,
            },
        )
        .await
        .expect("Failed to deactivate Newspack Newsletters plugin");
    ctx.api
        .plugins()
        .delete(&slug)
        .await
        .expect("Failed to uninstall Newspack Newsletters plugin");

    collection
        .fetch()
        .await
        .expect("Failed to fetch post types after plugin uninstallation");

    let post_types = collection
        .load_data()
        .await
        .expect("Failed to load post types from cache after plugin uninstallation");

    assert!(
        !post_types
            .iter()
            .any(|pt| pt.data.slug == "newspack_nl_cpt"),
        "Newspack Newsletters custom post type should not be present after plugin uninstallation"
    );

    RestoreServer::all().await;
}
