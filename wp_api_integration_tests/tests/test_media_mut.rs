use http::{HeaderMap, HeaderValue, Method};
use macro_helper::generate_update_test;
use reqwest::multipart::Part;
use serial_test::serial;
use wp_api::{
    media::{MediaCreateParams, MediaUpdateParams},
    posts::{PostCommentStatus, PostPingStatus, PostStatus},
    WpAuthentication, WpContentDisposition,
};
use wp_api_integration_tests::{
    api_client, backend::RestoreServer, AssertResponse, TestCredentials, FIRST_POST_ID,
    MEDIA_ID_611,
};

#[tokio::test]
#[serial]
//#[ignore]
async fn upload_media() {
    let authentication = WpAuthentication::from_username_and_password(
        TestCredentials::instance().admin_username.to_string(),
        TestCredentials::instance().admin_password.to_string(),
    );
    let client = reqwest::Client::new();
    let mut request = client
        .request(Method::POST, "http://localhost/wp-json/wp/v2/media/")
        .headers(header_map(&authentication));
    let mut jpeg_header_map = HeaderMap::new();
    jpeg_header_map.insert(
        http::header::CONTENT_TYPE,
        HeaderValue::from_str("image/jpeg").unwrap(),
    );
    let mut json_body_header_map = HeaderMap::new();
    json_body_header_map.insert(
        http::header::CONTENT_TYPE,
        HeaderValue::from_str("application/json").unwrap(),
    );
    let params = MediaCreateParams {
        title: Some("foo".to_string()),
        date: Some("2025-01-01T12:00:00".to_string()),
        ..Default::default()
    };
    let json_body = serde_json::to_vec(&params).unwrap();
    let form = reqwest::multipart::Form::new()
        .part(
            "file",
            Part::file("../sample.jpeg")
                .await
                .unwrap()
                .headers(jpeg_header_map),
        )
        //.part("title", Part::text("foooox"));
        // Setting the json doesn't seem to work. Each part can be set separately as shown above
        .part("data", Part::bytes(json_body).headers(json_body_header_map));
    request = request.multipart(form);

    println!("{:#?}", request);
    let response = request.send().await;

    println!("{:#?}", response);
    println!("{:#?}", response.unwrap().text().await);

    RestoreServer::db().await;
}

fn header_map(authentication: &WpAuthentication) -> HeaderMap {
    let mut header_map = HeaderMap::new();
    // set by reqwest multipart
    //header_map.insert(
    //    http::header::CONTENT_TYPE,
    //    HeaderValue::from_static("multipart/form-data"),
    //);
    header_map.insert(
        http::header::ACCEPT,
        HeaderValue::from_static("application/json"),
    );
    header_map.insert(
        http::header::CONTENT_DISPOSITION,
        HeaderValue::from_str("attachment;filename=\"sample.jpeg\"").unwrap(),
    );
    match authentication {
        WpAuthentication::None => (),
        WpAuthentication::AuthorizationHeader { ref token } => {
            let hv = HeaderValue::from_str(&format!("Basic {}", token)).unwrap();
            header_map.insert(http::header::AUTHORIZATION, hv);
        }
    };
    header_map
}

#[tokio::test]
#[serial]
//#[ignore]
async fn create_media() {
    api_client()
        .media()
        .create(
            &MediaCreateParams {
                title: Some("foo".to_string()),
                ..Default::default()
            },
            &WpContentDisposition::AttachmentFilepath("foo.jpeg".to_string()),
        )
        .await
        .assert_response();
    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
#[ignore]
async fn upload_media_from_client() {
    let response = api_client()
        .media()
        .upload(
            &MediaCreateParams {
                title: Some("Testing upload_media_from_client".to_string()),
                ..Default::default()
            },
            "../sample.jpeg".to_string(),
            "image/jpeg".to_string(),
        )
        .await;
    println!("{:#?}", response);
    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn delete_media() {
    // Delete the media using the API and ensure it's successful
    let media_delete_response = api_client().media().delete(&MEDIA_ID_611).await;
    assert!(
        media_delete_response.is_ok(),
        "{:#?}",
        media_delete_response
    );
    assert!(media_delete_response.unwrap().data.deleted);

    RestoreServer::db().await;
}

generate_update_test!(update_date, date, "2024-09-09T12:00:00".to_string());

generate_update_test!(update_date_gmt, date_gmt, "2024-09-09T12:00:00".to_string());

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

// TODO: `POST_TEMPLATE_SINGLE_WITH_SIDEBAR` doesn't work for `/media`.
//generate_update_test!(
//    update_template,
//    template,
//    POST_TEMPLATE_SINGLE_WITH_SIDEBAR.to_string()
//);

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
