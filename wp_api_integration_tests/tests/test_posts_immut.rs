use serial_test::parallel;
use wp_api::posts::PostListParams;
use wp_api_integration_tests::{api_client, AssertResponse};

#[tokio::test]
#[parallel]
async fn list_with_edit_context() {
    let posts = api_client()
        .posts()
        .list_with_edit_context(&PostListParams::default())
        .await
        .assert_response();
    println!("{:#?}", posts);
}

#[tokio::test]
#[parallel]
async fn list_with_embed_context() {
    let posts = api_client()
        .posts()
        .list_with_embed_context(&PostListParams::default())
        .await
        .assert_response();
    println!("{:#?}", posts);
}

#[tokio::test]
#[parallel]
async fn list_with_view_context() {
    let posts = api_client()
        .posts()
        .list_with_view_context(&PostListParams::default())
        .await
        .assert_response();
    println!("{:#?}", posts);
}
