use wp_api::{
    comments::{CommentId, CommentRetrieveParams},
    wp_com::endpoint::extensions::comments::WpComCommentExtensionProvider,
};
use wp_api_integration_tests::{api_client_backed_by_wp_com, prelude::*};

#[tokio::test]
#[parallel]
#[ignore]
async fn parse_extension_view_context() {
    let site_id = WpComTestCredentials::instance().site_id.to_string();
    let comment_id = CommentId(WpComTestCredentials::instance().comment_id);
    let client = api_client_backed_by_wp_com(site_id);

    let comment = client
        .comments()
        .retrieve_with_view_context(&comment_id, &CommentRetrieveParams::default())
        .await
        .assert_response()
        .data;
    assert!(
        comment
            .additional_fields
            .parse_wpcom_comments_extension()
            .is_ok()
    );
}

#[tokio::test]
#[parallel]
#[ignore]
async fn parse_extension_edit_context() {
    let site_id = WpComTestCredentials::instance().site_id.to_string();
    let comment_id = CommentId(WpComTestCredentials::instance().comment_id);
    let client = api_client_backed_by_wp_com(site_id);

    let comment = client
        .comments()
        .retrieve_with_edit_context(&comment_id, &CommentRetrieveParams::default())
        .await
        .assert_response()
        .data;
    assert!(
        comment
            .additional_fields
            .parse_wpcom_comments_extension()
            .is_ok()
    );
}

#[tokio::test]
#[parallel]
#[ignore]
async fn parse_extension_embed_context() {
    let site_id = WpComTestCredentials::instance().site_id.to_string();
    let comment_id = CommentId(WpComTestCredentials::instance().comment_id);
    let client = api_client_backed_by_wp_com(site_id);

    let comment = client
        .comments()
        .retrieve_with_embed_context(&comment_id, &CommentRetrieveParams::default())
        .await
        .assert_response()
        .data;
    assert!(
        comment
            .additional_fields
            .parse_wpcom_comments_extension()
            .is_ok()
    );
}
