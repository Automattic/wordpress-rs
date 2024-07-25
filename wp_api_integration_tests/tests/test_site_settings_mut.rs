use serial_test::serial;
use wp_api::site_settings::SiteSettingsUpdateParams;
use wp_api_integration_tests::{
    api_client,
    wp_cli::WpCliSiteSettings,
    wp_db::{self},
    AssertResponse,
};

#[tokio::test]
#[serial]
async fn update_site_settings() {
    wp_db::run_and_restore(|_db| async move {
        let new_title = "hello".to_string();
        let params = SiteSettingsUpdateParams {
            title: Some(new_title.clone()),
            ..Default::default()
        };
        let _updated_site_settings = api_client()
            .site_settings()
            .update(&params)
            .await
            .assert_response();
        let wp_cli_settings = WpCliSiteSettings::fetch().unwrap();
        assert_eq!(new_title, wp_cli_settings.title.unwrap());
    })
    .await;
}
