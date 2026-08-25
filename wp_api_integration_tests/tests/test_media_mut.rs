use macro_helper::generate_update_test;
use wp_api::date::WpDateString;
use wp_api::{
    media::{MediaCreateParams, MediaUpdateParams},
    posts::{PostCommentStatus, PostPingStatus, PostStatus},
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[serial]
async fn upload_media() {
    let title = "Foo media";
    let created_media = api_client()
        .media()
        .create(&MediaCreateParams {
            title: Some(title.to_string()),
            file_path: MEDIA_TEST_FILE_PATH.to_string(),
            ..Default::default()
        })
        .await
        .assert_response();
    assert_eq!(created_media.data.title.rendered.as_str(), title);
    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn delete_media() {
    // Delete the media using the API and ensure it's successful
    let media_delete_response = api_client().media().delete(&MEDIA_ID_611).await;
    assert!(media_delete_response.is_ok(), "{media_delete_response:#?}");
    assert!(media_delete_response.unwrap().data.deleted);

    RestoreServer::db().await;
}

generate_update_test!(
    update_date,
    date,
    WpDateString::new("2024-09-09T12:00:00".to_string())
);

generate_update_test!(
    update_date_gmt,
    date_gmt,
    unwrapped_wp_gmt_date_time("2024-09-09T12:00:00+00:00")
);

generate_update_test!(update_slug, slug, "new_slug".to_string());

generate_update_test!(update_status_to_draft, status, PostStatus::Draft);
generate_update_test!(update_status_to_future, status, PostStatus::Future);
generate_update_test!(update_status_to_pending, status, PostStatus::Pending);
generate_update_test!(update_status_to_private, status, PostStatus::Private);
generate_update_test!(update_status_to_publish, status, PostStatus::Publish);

generate_update_test!(update_title, title, "new_title".to_string());

generate_update_test!(
    update_comment_status_to_open,
    comment_status,
    PostCommentStatus::Open
);

generate_update_test!(
    update_comment_status_to_closed,
    comment_status,
    PostCommentStatus::Closed
);

generate_update_test!(
    update_ping_status_to_open,
    ping_status,
    PostPingStatus::Open
);

generate_update_test!(
    update_ping_status_to_closed,
    ping_status,
    PostPingStatus::Closed
);

generate_update_test!(update_alt_text, alt_text, "new_alt_text".to_string());

generate_update_test!(update_caption, caption, "new_caption".to_string());

generate_update_test!(
    update_description,
    description,
    "new_description".to_string()
);

generate_update_test!(update_post_id, post_id, FIRST_POST_ID);

async fn test_update_media(params: &MediaUpdateParams) {
    api_client()
        .media()
        .update(&MEDIA_ID_611, params)
        .await
        .assert_response();
    RestoreServer::db().await;
}

mod macro_helper {
    macro_rules! generate_update_test {
        ($ident:ident, $field:ident, $new_value:expr) => {
            paste::paste! {
                #[tokio::test]
                #[serial]
                async fn $ident() {
                    let updated_value = $new_value;
                    test_update_media(
                        &MediaUpdateParams {
                            $field: Some(updated_value),
                            ..Default::default()
                        })
                    .await;
                }
            }
        };
    }

    pub(super) use generate_update_test;
}
