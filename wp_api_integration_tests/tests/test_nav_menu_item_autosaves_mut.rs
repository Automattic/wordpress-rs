use wp_api::nav_menu_item_revisions::NavMenuItemRevisionCreateParams;
use wp_api::nav_menu_items::NavMenuItemId;
use wp_api_integration_tests::prelude::*;

#[tokio::test]
async fn create_autosave() {
    let params = NavMenuItemRevisionCreateParams::default();

    let autosave = api_client()
        .nav_menu_item_autosaves()
        .create(&nav_menu_item_id(), &params)
        .await
        .assert_response()
        .data;

    // Verify the autosave was created successfully - it should have an ID and parent reference
    assert_eq!(autosave.parent, nav_menu_item_id());

    RestoreServer::db().await;
}

fn nav_menu_item_id() -> NavMenuItemId {
    NavMenuItemId(TestCredentials::instance().nav_menu_item_id)
}
