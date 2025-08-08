use wp_api::{
    comments::{CommentId, CommentRetrieveParams},
};
use wp_api_integration_tests::{api_client_backed_by_wp_com, prelude::*, WpComTestCredentials};

#[tokio::test]
#[parallel]
#[ignore]
async fn parse_extension() {
    let site_id = WpComTestCredentials::instance().site_id.to_string();
    let comment_id = CommentId(WpComTestCredentials::instance().comment_id);
    let client = api_client_backed_by_wp_com(site_id);

    use wp_api::wp_com::endpoint::extensions::comments::WpComCommentExtensionProvider;

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
