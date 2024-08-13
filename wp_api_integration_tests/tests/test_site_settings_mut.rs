use serial_test::serial;
use wp_api::site_settings::{
    SiteSettingsCommentStatus, SiteSettingsPingStatus, SiteSettingsUpdateParams,
};
use wp_api_integration_tests::{api_client, AssertResponse, Backend, ServerRestore};

macro_rules! generate_test {
    ($ident:ident, $value:expr) => {
        generate_test!($ident, $value, $value);
    };
    ($ident:ident, $value:expr, $assertion_value:expr) => {
        paste::paste! {
            #[tokio::test]
            #[serial]
            async fn [<update_site_settings_ $ident>]() {
                let new_value = $value;
                let assertion_value = $assertion_value.to_string();
                // First assert that the new value is not the same as the old value to avoid
                // false positive assertion
                assert_ne!(Some(assertion_value.clone()), Backend::site_settings().await.unwrap().$ident);
                let params = SiteSettingsUpdateParams {
                    $ident: Some(new_value.clone()),
                    ..Default::default()
                };
                let _updated_site_settings = api_client()
                    .site_settings()
                    .update(&params)
                    .await
                    .assert_response();
                // Assert that the value was updated to the new one
                assert_eq!(Some(assertion_value), Backend::site_settings().await.unwrap().$ident);

                ServerRestore::db().await;
            }
        }
    };
}

generate_test!(title, "new_title".to_string());
generate_test!(description, "new_description".to_string());
generate_test!(url, "https://example.com".to_string());
generate_test!(email, "foo@example.com".to_string());
generate_test!(timezone, "EST".to_string());
generate_test!(date_format, "YYYY-MM-DDTHH".to_string());
generate_test!(time_format, "mm:ss.sssZ".to_string());
generate_test!(start_of_week, 5);
generate_test!(language, "en_CA".to_string());
generate_test!(use_smilies, false, "");
generate_test!(default_category, 2);
generate_test!(default_post_format, "new_post_format".to_string());
generate_test!(posts_per_page, 27);
generate_test!(show_on_front, "new_front".to_string());
generate_test!(page_on_front, 2);
generate_test!(page_for_posts, 2);
generate_test!(default_ping_status, SiteSettingsPingStatus::Closed);
generate_test!(default_comment_status, SiteSettingsCommentStatus::Closed);
generate_test!(site_logo, 1);
generate_test!(site_icon, 1);
